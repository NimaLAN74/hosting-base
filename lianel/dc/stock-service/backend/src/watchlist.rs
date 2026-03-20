//! Watchlist: fixed list of symbols, current price from IBKR only. Refreshed every 60s via /iserver/marketdata/snapshot.

use crate::ibkr::IbkrOAuthClient;
use crate::today_cache;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing;

const REFRESH_INTERVAL_SECS: u64 = 60;
/// 31 = last; 84/86 = bid/ask (used when last is empty — delayed data / pre-flight).
const SNAPSHOT_FIELDS: &str = "31,84,86";

/// Default 10 common symbols (US large cap). Conids from IBKR for these symbols.
pub const DEFAULT_SYMBOLS: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "JPM", "V", "JNJ",
];

/// Symbol → conid for default watchlist (IBKR US stock conids).
/// Refreshed from `/trsrv/stocks` each cycle when possible; these are fallbacks if trsrv fails.
fn default_symbol_conids() -> HashMap<String, u64> {
    let mut m = HashMap::new();
    m.insert("AAPL".to_string(), 265598);
    m.insert("MSFT".to_string(), 272093);
    m.insert("GOOGL".to_string(), 15124834);
    m.insert("AMZN".to_string(), 3691937);
    m.insert("META".to_string(), 107113172);
    m.insert("NVDA".to_string(), 4815747);
    m.insert("TSLA".to_string(), 76792991);
    m.insert("JPM".to_string(), 4722877);
    m.insert("V".to_string(), 27674840);
    m.insert("JNJ".to_string(), 4362728);
    m
}

fn norm_sym(s: &str) -> String {
    s.trim().to_ascii_uppercase()
}

/// True if `symbol` is one of the fixed watchlist tickers.
pub fn is_watchlist_symbol(symbol: &str) -> bool {
    let n = norm_sym(symbol);
    DEFAULT_SYMBOLS.iter().any(|s| norm_sym(s) == n)
}

/// Static fallback conid (for tests and last-resort). Prefer [`get_conid_with_cache`] after a refresh.
pub fn get_conid_for_symbol(symbol: &str) -> Option<u64> {
    default_symbol_conids().get(&norm_sym(symbol)).copied()
}

/// Conid from last successful trsrv resolution, else static fallback.
pub fn get_conid_with_cache(symbol: &str, cache: &WatchlistCache) -> Option<u64> {
    let n = norm_sym(symbol);
    cache
        .resolved_conids
        .get(&n)
        .copied()
        .or_else(|| get_conid_for_symbol(&n))
}

/// Resolve symbols to conids via IBKR GET /trsrv/stocks.
///
/// In practice this endpoint requires OAuth in our environment; without Authorization it returns 401 and we'd
/// fall back to hardcoded (possibly stale) conids.
async fn fetch_conids_from_trsrv(
    ibkr_client: &IbkrOAuthClient,
    base_url: &str,
    symbols: &[&str],
) -> HashMap<String, u64> {
    let symbols_param = symbols.join(",");
    let url = format!(
        "{}/trsrv/stocks?symbols={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(&symbols_param)
    );

    // OAuth signature + (often) an active brokerage session cookie are required for /trsrv/stocks.
    // Without the cookie, IBKR can return 401/empty and we fall back to hardcoded conids.
    let auth = match ibkr_client.sign_request("GET", &url, None).await {
        Ok(a) => Some(a),
        Err(_) => None,
    };
    let cookie = ibkr_client.get_session_for_cookie().await.ok();
    if auth.is_none() {
        tracing::warn!("trsrv/stocks: sign_request failed (falling back to default conids)");
    }
    if cookie.is_none() {
        tracing::warn!("trsrv/stocks: get_session_for_cookie failed (no api cookie; falling back to default conids)");
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("trsrv/stocks: failed building http client: {}", e);
            return default_symbol_conids();
        }
    };

    let mut req = client.get(&url).header("User-Agent", "Console");
    if let Some(a) = auth {
        req = req.header("Authorization", a.as_str());
    }
    if let Some(c) = cookie {
        req = req.header("Cookie", format!("api={}", c));
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("trsrv/stocks request failed: {}", e);
            return default_symbol_conids();
        }
    };

    let status = resp.status();
    let body_text = match resp.text().await {
        Ok(b) => b,
        Err(_) => {
            tracing::warn!("trsrv/stocks: failed reading response body (status {})", status);
            return default_symbol_conids();
        }
    };
    if !status.is_success() {
        let snippet: String = body_text
            .trim_start()
            .chars()
            .take(220)
            .collect();
        tracing::warn!(
            "trsrv/stocks: non-success status={} body_snippet={}",
            status,
            snippet
        );
        return default_symbol_conids();
    }
    let parsed: serde_json::Value = match serde_json::from_str(&body_text) {
        Ok(v) => v,
        Err(e) => {
            let snippet: String = body_text
                .trim_start()
                .chars()
                .take(220)
                .collect();
            tracing::warn!(
                "trsrv/stocks: JSON parse failed: {} body_snippet={}",
                e,
                snippet
            );
            return default_symbol_conids();
        }
    };
    let mut out = default_symbol_conids();
    // Response can be object with symbol keys -> array of {conid}, or array of {symbol, conid}
    if let Some(obj) = parsed.as_object() {
        for (raw_key, val) in obj {
            let kn = norm_sym(raw_key);
            for sym in symbols {
                if norm_sym(sym) != kn {
                    continue;
                }
                match conid_from_trsrv_entry(val) {
                    Some(c) => {
                        out.insert((*sym).to_string(), c);
                    }
                    None => {
                        tracing::warn!(
                            "trsrv/stocks: could not extract conid for symbol={} (raw_key={})",
                            sym,
                            raw_key
                        );
                    }
                }
                break;
            }
        }
    } else if let Some(arr) = parsed.as_array() {
        for item in arr {
            let sym_str = item
                .get("symbol")
                .or_else(|| item.get("Symbol"))
                .and_then(|v| v.as_str());
            let Some(sym_str) = sym_str else {
                continue;
            };
            let sn = norm_sym(sym_str);
            let Some(matched) = symbols.iter().find(|s| norm_sym(s) == sn) else {
                continue;
            };
            match conid_from_trsrv_entry(item) {
                Some(c) => {
                    out.insert((*matched).to_string(), c);
                }
                None => {
                    tracing::warn!(
                        "trsrv/stocks: could not extract conid for symbol={} (item_symbol={})",
                        matched,
                        sym_str
                    );
                }
            }
        }
    }
    out
}

fn conid_from_trsrv_entry(val: &serde_json::Value) -> Option<u64> {
    let node = if let Some(arr) = val.as_array() {
        arr.first()?
    } else {
        val
    };
    node
        .get("conid")
        .and_then(|v| v.as_u64())
        .or_else(|| node.get("conid").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
}

#[derive(Clone, Debug, Serialize)]
pub struct WatchlistQuote {
    pub symbol: String,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub updated_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WatchlistResponse {
    pub symbols: Vec<WatchlistQuote>,
    pub as_of: String,
    pub provider: String,
}

pub struct WatchlistCache {
    pub quotes: HashMap<String, WatchlistQuote>,
    pub as_of: String,
    /// Latest symbol → conid from `/trsrv/stocks` (merged with defaults). Used for `/history` and `/today`.
    pub resolved_conids: HashMap<String, u64>,
}

impl Default for WatchlistCache {
    fn default() -> Self {
        let symbols: Vec<WatchlistQuote> = DEFAULT_SYMBOLS
            .iter()
            .map(|s| WatchlistQuote {
                symbol: (*s).to_string(),
                price: None,
                currency: Some("USD".to_string()),
                updated_at: None,
                error: Some("pending".to_string()),
            })
            .collect();
        let as_of = iso_ts();
        let quotes = symbols
            .into_iter()
            .map(|q| (q.symbol.clone(), q))
            .collect();
        Self {
            quotes,
            as_of,
            resolved_conids: default_symbol_conids(),
        }
    }
}

fn iso_ts() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let rem = secs % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    let y = 1970u64 + (days / 365).min(200);
    let d = (days % 365) as u32;
    let mo = ((d / 31) % 12).max(1).min(12);
    let da = (d % 28).max(1).min(28);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, da, h, m, s)
}

/// Refresh watchlist from IBKR only. If ibkr_client is None, all quotes get error "IBKR not configured".
/// When redis is Some, each symbol's price is pushed to Redis for today's intraday cache.
pub async fn refresh_from_ibkr(
    cache: Arc<RwLock<WatchlistCache>>,
    ibkr_client: Option<Arc<IbkrOAuthClient>>,
    base_url: &str,
    redis: Option<&redis::aio::ConnectionManager>,
) {
    // Resolve symbols to conids via IBKR so we use correct conids (fixes missing prices for some symbols).
    // If it fails, we keep using the hardcoded defaults.
    let conids_map = if let Some(ref client) = ibkr_client {
        fetch_conids_from_trsrv(client.as_ref(), base_url, DEFAULT_SYMBOLS).await
    } else {
        default_symbol_conids()
    };
    let conids_list: Vec<u64> = DEFAULT_SYMBOLS
        .iter()
        .filter_map(|s| conids_map.get(*s).copied())
        .collect();
    let conids_param = conids_list
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let as_of = iso_ts();

    let Some(client) = ibkr_client else {
        let mut next = WatchlistCache::default();
        next.as_of = as_of.clone();
        for s in DEFAULT_SYMBOLS {
            next.quotes.insert(
                (*s).to_string(),
                WatchlistQuote {
                    symbol: (*s).to_string(),
                    price: None,
                    currency: Some("USD".to_string()),
                    updated_at: Some(as_of.clone()),
                    error: Some("IBKR not configured".to_string()),
                },
            );
        }
        let mut g = cache.write().await;
        *g = next;
        return;
    };

    let url = format!(
        "{}/iserver/marketdata/snapshot?conids={}&fields={}",
        base_url.trim_end_matches('/'),
        conids_param,
        SNAPSHOT_FIELDS
    );

    let auth = if client.use_oauth_for_api() {
        match client.sign_request("GET", &url, None).await {
            Ok(a) => Some(a),
            Err(e) => {
                tracing::warn!("Watchlist IBKR sign_request failed: {}", e);
                let mut next = WatchlistCache::default();
                next.as_of = as_of.clone();
                for s in DEFAULT_SYMBOLS {
                    next.quotes.insert(
                        (*s).to_string(),
                        WatchlistQuote {
                            symbol: (*s).to_string(),
                            price: None,
                            currency: Some("USD".to_string()),
                            updated_at: Some(as_of.clone()),
                            error: Some(format!("IBKR auth failed: {}", e)),
                        },
                    );
                }
                let mut g = cache.write().await;
                *g = next;
                return;
            }
        }
    } else {
        None
    };

    // /iserver/* requires brokerage session cookie (from tickle or Gateway cookie mode)
    let cookie = match client.get_session_for_cookie().await {
        Ok(session) => format!("api={}", session),
        Err(e) => {
            tracing::warn!("Watchlist IBKR tickle (session) failed: {}", e);
            let mut next = WatchlistCache::default();
            next.as_of = as_of.clone();
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(format!("IBKR session failed (tickle): {}", e)),
                    },
                );
            }
            let mut g = cache.write().await;
            *g = next;
            return;
        }
    };

    let http_client = client
        .http_client()
        .unwrap_or_else(|_| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default()
        });

    // Try to ensure brokerage session is authenticated before /accounts + marketdata.
    // IBKR suggests: GET /sso/validate then POST /iserver/reauthenticate when auth is false.
    let validate_url = format!("{}/sso/validate", base_url.trim_end_matches('/'));
    let validate_req = http_client
        .get(&validate_url)
        .header("Cookie", &cookie)
        .header("User-Agent", "Console");
    let validate_req = if let Some(ref a) = auth {
        validate_req.header("Authorization", a.as_str())
    } else {
        validate_req
    };
    if let Err(e) = validate_req.send().await {
        tracing::warn!("Watchlist IBKR /sso/validate failed: {}", e);
    }

    let reauth_url = format!("{}/iserver/reauthenticate", base_url.trim_end_matches('/'));
    let reauth_req = http_client
        .post(&reauth_url)
        .header("Cookie", &cookie)
        .header("User-Agent", "Console")
        .header("Content-Length", "0")
        .body("");
    let reauth_req = if let Some(ref a) = auth {
        reauth_req.header("Authorization", a.as_str())
    } else {
        reauth_req
    };
    if let Ok(r) = reauth_req.send().await {
        let status = r.status();
        if !status.is_success() {
            let body = r.text().await.unwrap_or_default();
            tracing::warn!(
                "Watchlist IBKR /iserver/reauthenticate returned {}: {}",
                status,
                body.trim_start().chars().take(200).collect::<String>()
            );
        }
    }

    // IBKR requires /iserver/accounts to be queried before market data snapshot.
    let accounts_url = format!("{}/iserver/accounts", base_url.trim_end_matches('/'));
    let accounts_req = http_client
        .get(&accounts_url)
        .header("Cookie", &cookie)
        .header("User-Agent", "Console")
        .header("Accept", "application/json");
    let accounts_req = if let Some(ref a) = auth {
        accounts_req.header("Authorization", a.as_str())
    } else {
        accounts_req
    };
    match accounts_req.send().await {
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            if !status.is_success() {
                tracing::warn!(
                    "Watchlist IBKR /accounts pre-call failed ({}): {}",
                    status,
                    body.trim_start().chars().take(200).collect::<String>()
                );
                let mut next = WatchlistCache::default();
                next.as_of = as_of.clone();
                for s in DEFAULT_SYMBOLS {
                    next.quotes.insert(
                        (*s).to_string(),
                        WatchlistQuote {
                            symbol: (*s).to_string(),
                            price: None,
                            currency: Some("USD".to_string()),
                            updated_at: Some(as_of.clone()),
                            error: Some(format!(
                                "IBKR /accounts failed {}: {}",
                                status,
                                body.trim_start().chars().take(200).collect::<String>()
                            )),
                        },
                    );
                }
                let mut g = cache.write().await;
                *g = next;
                return;
            }
        }
        Err(e) => {
            tracing::warn!("Watchlist IBKR /accounts pre-call request error: {}", e);
            let mut next = WatchlistCache::default();
            next.as_of = as_of.clone();
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(format!("IBKR /accounts request failed: {}", e)),
                    },
                );
            }
            let mut g = cache.write().await;
            *g = next;
            return;
        }
    }
    let req = http_client.get(&url).header("Cookie", cookie).header("User-Agent", "Console");
    let req = if let Some(ref a) = auth {
        req.header("Authorization", a.as_str())
    } else {
        req
    };
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Watchlist IBKR snapshot request failed: {}", e);
            let mut next = WatchlistCache::default();
            next.as_of = as_of.clone();
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(format!("Request failed: {}", e)),
                    },
                );
            }
            let mut g = cache.write().await;
            *g = next;
            return;
        }
    };

    let status = resp.status();
    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Watchlist IBKR snapshot body read failed (status {}): {}", status, e);
            let mut next = WatchlistCache::default();
            next.as_of = as_of.clone();
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(format!("IBKR response read failed: {}", e)),
                    },
                );
            }
            let mut g = cache.write().await;
            *g = next;
            return;
        }
    };

    let body_parsed: Result<serde_json::Value, _> = serde_json::from_str(body_text.trim());
    let mut next = WatchlistCache::default();
    next.as_of = as_of.clone();

    let conid_to_symbol: HashMap<u64, String> = conids_map
        .iter()
        .map(|(sym, &c)| (c, sym.clone()))
        .collect();

    fn parse_snapshot_f64(item: &serde_json::Value, field: &str) -> Option<f64> {
        item.get(field).and_then(|v| v.as_f64()).or_else(|| {
            item.get(field)
                .and_then(|v| v.as_str())
                .and_then(|p| p.replace(',', "").parse::<f64>().ok())
        })
    }

    // Prefer last (31); if missing use bid/ask midpoint or single side (delayed / pre-flight).
    fn best_price_from_snapshot_item(item: &serde_json::Value) -> Option<f64> {
        let last = parse_snapshot_f64(item, "31").filter(|p| p.is_finite() && *p > 0.0);
        if last.is_some() {
            return last;
        }
        let bid = parse_snapshot_f64(item, "84").filter(|p| p.is_finite() && *p > 0.0);
        let ask = parse_snapshot_f64(item, "86").filter(|p| p.is_finite() && *p > 0.0);
        match (bid, ask) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            _ => None,
        }
    }

    // Extract error message from IBKR error object, e.g. {"error":"...","statusCode":400}
    fn ibkr_error_from_value(v: &serde_json::Value) -> Option<String> {
        let obj = v.as_object()?;
        let msg = obj
            .get("error")
            .and_then(|e| e.as_str())
            .map(String::from)
            .or_else(|| obj.get("message").and_then(|m| m.as_str()).map(String::from));
        let code = obj.get("statusCode").and_then(|c| c.as_u64());
        match (msg, code) {
            (Some(m), Some(c)) => Some(format!("IBKR (status {}): {}", c, m)),
            (Some(m), None) => Some(format!("IBKR: {}", m)),
            (None, Some(c)) => Some(format!("IBKR error (status {})", c)),
            (None, None) => None,
        }
    }

    match body_parsed {
        Ok(serde_json::Value::Array(arr)) => {
            for item in arr {
                let conid = item.get("conid").and_then(|v| v.as_u64()).or_else(|| {
                    item.get("conid").and_then(|v| v.as_str()).and_then(|s| s.parse().ok())
                });
                let mut symbol = conid
                    .and_then(|c| conid_to_symbol.get(&c).cloned())
                    .unwrap_or_else(|| "?".to_string());
                if symbol == "?" {
                    if let Some(s) = item
                        .get("55")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                    {
                        symbol = s;
                    }
                }
                let price = best_price_from_snapshot_item(&item);
                let error = if price.is_none() {
                    Some(
                        "no price (needs market data subscription, or pre-flight / delayed quote)"
                            .to_string(),
                    )
                } else {
                    None
                };
                next.quotes.insert(
                    symbol.clone(),
                    WatchlistQuote {
                        symbol,
                        price,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error,
                    },
                );
            }
            for s in DEFAULT_SYMBOLS {
                if !next.quotes.contains_key(*s) {
                    next.quotes.insert(
                        (*s).to_string(),
                        WatchlistQuote {
                            symbol: (*s).to_string(),
                            price: None,
                            currency: Some("USD".to_string()),
                            updated_at: Some(as_of.clone()),
                            error: Some("no data from IBKR".to_string()),
                        },
                    );
                }
            }
        }
        Ok(v @ serde_json::Value::Object(_)) => {
            // 400/4xx/5xx often return JSON object like {"error":"...","statusCode":400}
            let err_msg = ibkr_error_from_value(&v)
                .unwrap_or_else(|| format!("IBKR error (status {})", status));
            tracing::warn!("Watchlist IBKR snapshot error response: {}", err_msg);
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(err_msg.clone()),
                    },
                );
            }
        }
        Ok(_) => {
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(format!("IBKR unexpected response (status {})", status)),
                    },
                );
            }
        }
        Err(e) => {
            // Try to parse as object to get error message even when we expected array
            let obj_err = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(body_text.trim())
                .ok()
                .and_then(|m| ibkr_error_from_value(&serde_json::Value::Object(m)));
            let err_msg = obj_err.unwrap_or_else(|| {
                let hint = if body_text.trim().is_empty() {
                    "empty response (check IBKR session/cookie)"
                } else if body_text.trim().starts_with('<') {
                    "HTML response (IBKR may require session cookie or login)"
                } else {
                    "unexpected response format"
                };
                format!("IBKR {} (status {}): {}", hint, status, e)
            });
            tracing::warn!("Watchlist IBKR snapshot parse failed: {}", err_msg);
            for s in DEFAULT_SYMBOLS {
                next.quotes.insert(
                    (*s).to_string(),
                    WatchlistQuote {
                        symbol: (*s).to_string(),
                        price: None,
                        currency: Some("USD".to_string()),
                        updated_at: Some(as_of.clone()),
                        error: Some(err_msg.clone()),
                    },
                );
            }
        }
    }

    next.resolved_conids = conids_map;

    if let Some(conn) = redis {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        for (symbol, q) in &next.quotes {
            if let Some(price) = q.price {
                today_cache::push_price(Some(conn), symbol, now_secs, price).await;
            }
        }
    }

    let mut g = cache.write().await;
    *g = next;
}

/// Fetch raw IBKR `/iserver/marketdata/snapshot` response for the given conids.
/// Used for debugging missing prices (what fields IBKR actually returns).
pub async fn fetch_snapshot_raw_for_conids(
    ibkr_client: &IbkrOAuthClient,
    base_url: &str,
    conids: &[u64],
    fields: Option<&str>,
) -> Result<serde_json::Value, String> {
    if conids.is_empty() {
        return Err("conids required".to_string());
    }

    let conids_param = conids
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let url = match fields {
        Some(f) if !f.trim().is_empty() => format!(
            "{}/iserver/marketdata/snapshot?conids={}&fields={}",
            base_url.trim_end_matches('/'),
            conids_param,
            f
        ),
        _ => format!(
            "{}/iserver/marketdata/snapshot?conids={}",
            base_url.trim_end_matches('/'),
            conids_param
        ),
    };

    // Build OAuth Authorization header if we're not in gateway-cookie mode.
    let auth = if ibkr_client.use_oauth_for_api() {
        Some(
            ibkr_client
                .sign_request("GET", &url, None)
                .await
                .map_err(|e| format!("IBKR sign_request failed: {e}"))?,
        )
    } else {
        None
    };

    // /iserver/* requires brokerage session cookie (from tickle or Gateway cookie mode).
    let cookie = ibkr_client
        .get_session_for_cookie()
        .await
        .map_err(|e| format!("IBKR get_session_for_cookie failed: {e}"))?;

    let http_client = ibkr_client.http_client().map_err(|e| e.to_string()).or_else(|_| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())
    })?;

    // Try to ensure brokerage session is authenticated before /accounts + market data snapshot.
    let validate_url = format!("{}/sso/validate", base_url.trim_end_matches('/'));
    let validate_req = http_client
        .get(&validate_url)
        .header("Cookie", &format!("api={cookie}"))
        .header("User-Agent", "Console");
    let validate_req = if let Some(ref a) = auth {
        validate_req.header("Authorization", a.as_str())
    } else {
        validate_req
    };
    let _ = validate_req.send().await;

    let reauth_url = format!("{}/iserver/reauthenticate", base_url.trim_end_matches('/'));
    let reauth_req = http_client
        .post(&reauth_url)
        .header("Cookie", &format!("api={cookie}"))
        .header("User-Agent", "Console")
        .header("Content-Length", "0")
        .body("");
    let reauth_req = if let Some(ref a) = auth {
        reauth_req.header("Authorization", a.as_str())
    } else {
        reauth_req
    };
    let _ = reauth_req.send().await;

    // IBKR requires /iserver/accounts to be queried before market data snapshot.
    let accounts_url = format!("{}/iserver/accounts", base_url.trim_end_matches('/'));
    let accounts_req = http_client
        .get(&accounts_url)
        .header("Cookie", &format!("api={cookie}"))
        .header("User-Agent", "Console")
        .header("Accept", "application/json");
    let accounts_req = if let Some(ref a) = auth {
        accounts_req.header("Authorization", a.as_str())
    } else {
        accounts_req
    };
    let accounts_resp = accounts_req.send().await.map_err(|e| format!("IBKR /accounts req failed: {e}"))?;
    if !accounts_resp.status().is_success() {
        let status = accounts_resp.status();
        let body = accounts_resp.text().await.unwrap_or_default();
        return Err(format!("IBKR /accounts failed ({status}): {body}"));
    }

    let req = http_client
        .get(&url)
        .header("Cookie", &format!("api={cookie}"))
        .header("User-Agent", "Console");
    let req = if let Some(ref a) = auth {
        req.header("Authorization", a.as_str())
    } else {
        req
    };

    let resp = req
        .send()
        .await
        .map_err(|e| format!("IBKR snapshot request failed: {e}"))?;
    let status = resp.status();
    let body_text = resp.text().await.map_err(|e| format!("IBKR snapshot body read failed: {e}"))?;

    serde_json::from_str(body_text.trim())
        .map_err(|e| format!("IBKR snapshot JSON parse failed (status {status}): {e}; body starts: {}", body_text.trim_start().chars().take(200).collect::<String>()))
}

/// Spawn the background task that refreshes the watchlist every REFRESH_INTERVAL_SECS (IBKR only).
pub fn spawn_watchlist_ticker(
    cache: Arc<RwLock<WatchlistCache>>,
    ibkr_client: Option<Arc<IbkrOAuthClient>>,
    base_url: String,
    redis: Option<redis::aio::ConnectionManager>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(REFRESH_INTERVAL_SECS));
        interval.tick().await;
        loop {
            refresh_from_ibkr(
                Arc::clone(&cache),
                ibkr_client.clone(),
                &base_url,
                redis.as_ref(),
            )
            .await;
            interval.tick().await;
        }
    });
}

/// Fallback response when cache is unavailable (avoids 502).
pub fn fallback_response() -> WatchlistResponse {
    let symbols: Vec<WatchlistQuote> = DEFAULT_SYMBOLS
        .iter()
        .map(|s| WatchlistQuote {
            symbol: (*s).to_string(),
            price: None,
            currency: Some("USD".to_string()),
            updated_at: None,
            error: Some("Service temporarily unavailable".to_string()),
        })
        .collect();
    WatchlistResponse {
        symbols,
        as_of: iso_ts(),
        provider: "IBKR".to_string(),
    }
}

/// Build the API response from current cache. Uses fallback if cache read times out (e.g. ticker holding lock).
pub async fn get_watchlist_response(cache: &RwLock<WatchlistCache>) -> WatchlistResponse {
    let read_guard = match tokio::time::timeout(Duration::from_secs(5), cache.read()).await {
        Ok(g) => g,
        Err(_) => {
            tracing::warn!("Watchlist cache read timed out");
            return fallback_response();
        }
    };
    let g = read_guard;
    let symbols: Vec<WatchlistQuote> = DEFAULT_SYMBOLS
        .iter()
        .map(|s| {
            g.quotes.get(*s).cloned().unwrap_or_else(|| WatchlistQuote {
                symbol: (*s).to_string(),
                price: None,
                currency: Some("USD".to_string()),
                updated_at: None,
                error: Some("pending".to_string()),
            })
        })
        .collect();
    WatchlistResponse {
        symbols,
        as_of: g.as_of.clone(),
        provider: "IBKR".to_string(),
    }
}
