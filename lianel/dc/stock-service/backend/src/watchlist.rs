//! Watchlist: fixed list of symbols, current price from IBKR only. Refreshed every 60s via /iserver/marketdata/snapshot.

use crate::ibkr::IbkrOAuthClient;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing;

const REFRESH_INTERVAL_SECS: u64 = 60;
/// Field 31 = last price (IBKR snapshot).
const SNAPSHOT_FIELDS: &str = "31";

/// Default 10 common symbols (US large cap). Conids from IBKR for these symbols.
pub const DEFAULT_SYMBOLS: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "JPM", "V", "JNJ",
];

/// Symbol → conid for default watchlist (IBKR US stock conids).
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
        Self { quotes, as_of }
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
pub async fn refresh_from_ibkr(
    cache: Arc<RwLock<WatchlistCache>>,
    ibkr_client: Option<Arc<IbkrOAuthClient>>,
    base_url: &str,
) {
    let conids_map = default_symbol_conids();
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
    // IBKR requires /iserver/accounts to be queried before market data snapshot.
    let accounts_url = format!("{}/iserver/accounts", base_url.trim_end_matches('/'));
    let accounts_req = http_client
        .get(&accounts_url)
        .header("Cookie", &cookie)
        .header("User-Agent", "Console");
    let accounts_req = if let Some(ref a) = auth {
        accounts_req.header("Authorization", a.as_str())
    } else {
        accounts_req
    };
    if let Err(e) = accounts_req.send().await {
        tracing::warn!("Watchlist IBKR /accounts pre-call failed: {}", e);
    }
    let req = http_client.get(&url).header("Cookie", cookie).header("User-Agent", "Console");
    let req = if let Some(ref a) = auth {
        req.header("Authorization", a.as_str())
    } else {
        req
    };
    let resp = match req.send().await
    {
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

    /// Extract error message from IBKR error object, e.g. {"error":"...","statusCode":400}
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
                let symbol = conid
                    .and_then(|c| conid_to_symbol.get(&c).cloned())
                    .unwrap_or_else(|| "?".to_string());
                // Field 31 = last price; IBKR may return number or string
                let price = item.get("31").and_then(|v| v.as_f64()).or_else(|| {
                    item.get("31")
                        .and_then(|v| v.as_str())
                        .and_then(|p| p.replace(',', "").parse::<f64>().ok())
                });
                let error = if price.is_none() {
                    Some("no price (pre-flight or stream not ready)".to_string())
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

    let mut g = cache.write().await;
    *g = next;
}

/// Spawn the background task that refreshes the watchlist every REFRESH_INTERVAL_SECS (IBKR only).
pub fn spawn_watchlist_ticker(
    cache: Arc<RwLock<WatchlistCache>>,
    ibkr_client: Option<Arc<IbkrOAuthClient>>,
    base_url: String,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(REFRESH_INTERVAL_SECS));
        interval.tick().await;
        loop {
            refresh_from_ibkr(Arc::clone(&cache), ibkr_client.clone(), &base_url).await;
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
