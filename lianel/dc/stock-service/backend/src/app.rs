//! Stock service: minimal API for SSO verification and IBKR authentication. Health, status, /me, /api/v1/ibkr/verify.

use axum::{
    extract::{Query, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::auth::{KeycloakJwtValidator, UserId};
use crate::daily_strategy;
use crate::ibkr::IbkrOAuthClient;
use crate::watchlist;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub validator: Arc<KeycloakJwtValidator>,
    pub config: Option<Arc<crate::config::AppConfig>>,
    pub ibkr_client: Option<Arc<IbkrOAuthClient>>,
    pub watchlist_cache: Arc<RwLock<watchlist::WatchlistCache>>,
    pub redis: Option<redis::aio::ConnectionManager>,
}

#[derive(Clone)]
struct UserDisplayName(pub String);

#[derive(Clone)]
struct UserEmail(pub Option<String>);

fn looks_like_user_id(candidate: &str) -> bool {
    let s = candidate.trim();
    if s.is_empty() {
        return true;
    }
    let uuid_like = s.len() == 36
        && s.chars().filter(|c| *c == '-').count() == 4
        && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let tokenish = s.contains('|') || s.starts_with("auth0|");
    uuid_like || tokenish
}

fn normalize_display_name(
    display_name: Option<String>,
    user_id: &str,
    email: Option<&str>,
) -> Option<String> {
    let cleaned = display_name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .filter(|v| v != user_id)
        .filter(|v| !looks_like_user_id(v));
    if cleaned.is_some() {
        return cleaned;
    }
    email
        .and_then(|v| v.split('@').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(String::from)
}

pub fn create_router(state: AppState) -> Router {
    // Public routes: canonical /api/v1/* and prefixed /api/v1/stock-service/* so nginx/proxy quirks never 404.
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/watchlist", get(watchlist_handler))
        .route("/api/v1/stock-service/watchlist", get(watchlist_handler))
        .route("/api/v1/history", get(history_handler))
        .route("/api/v1/stock-service/history", get(history_handler))
        .route("/api/v1/today", get(today_handler))
        .route("/api/v1/stock-service/today", get(today_handler))
        .route("/api/v1/daily-signals", get(daily_signals_handler))
        .route("/api/v1/stock-service/daily-signals", get(daily_signals_handler))
        .route("/api/v1/daily-signals/model", get(daily_signals_model_handler))
        .route(
            "/api/v1/stock-service/daily-signals/model",
            get(daily_signals_model_handler),
        )
        .route("/api/v1/daily-signals/research", get(daily_signals_research_handler))
        .route(
            "/api/v1/stock-service/daily-signals/research",
            get(daily_signals_research_handler),
        )
        // nginx rewrites `/api/v1/stock-service/(.*)` → `/api/v1/$1`
        // so we also need the canonical alias for debug.
        .route("/api/v1/debug/snapshot", get(debug_snapshot_handler))
        .route(
            "/api/v1/stock-service/debug/snapshot",
            get(debug_snapshot_handler),
        );

    let protected = Router::new()
        .route("/api/v1/me", get(me))
        .route("/api/v1/ibkr/verify", get(ibkr_verify))
        .route("/api/v1/ibkr/gateway-cookie", post(set_gateway_cookie))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let parse_bearer = |raw: &str| {
        raw.strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };

    let headers = req.headers();
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bearer)
        .or_else(|| {
            headers
                .get("x-auth-request-access-token")
                .and_then(|v| v.to_str().ok())
                .and_then(|raw| {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        None
                    } else if let Some(parsed) = parse_bearer(trimmed) {
                        Some(parsed)
                    } else {
                        Some(trimmed.to_string())
                    }
                })
        });

    if let Some(token) = token {
        let identity = match state.validator.validate_bearer_identity_via_userinfo(&token).await {
            Ok(id) => Ok(id),
            Err(e_userinfo) => {
                tracing::debug!("Userinfo validation failed ({}), trying JWKS", e_userinfo);
                state.validator.validate_bearer_identity(&token).await
            }
        };
        match identity {
            Ok(identity) => {
                let display_name = normalize_display_name(
                    identity.display_name.clone(),
                    &identity.user_id,
                    identity.email.as_deref(),
                )
                .unwrap_or_else(|| "User".to_string());
                req.extensions_mut().insert(UserId(identity.user_id));
                req.extensions_mut().insert(UserDisplayName(display_name));
                req.extensions_mut().insert(UserEmail(identity.email));
                return next.run(req).await;
            }
            Err(e) => {
                tracing::warn!("Token validation failed (token present): {}", e);
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Invalid or expired token",
                        "detail": e.to_string()
                    })),
                )
                    .into_response();
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "Missing authorization token",
            "detail": "Send Authorization: Bearer <token> or use x-auth-request-access-token"
        })),
    )
        .into_response()
}

async fn health() -> &'static str {
    "ok"
}

async fn watchlist_handler(State(state): State<AppState>) -> Json<watchlist::WatchlistResponse> {
    let response = watchlist::get_watchlist_response(&state.watchlist_cache).await;
    Json(response)
}

#[derive(serde::Deserialize)]
struct HistoryQuery {
    symbol: String,
    #[serde(default)]
    days: Option<u32>,
}

#[derive(serde::Deserialize)]
struct TodayQuery {
    symbol: String,
}

async fn today_handler(
    State(state): State<AppState>,
    Query(q): Query<TodayQuery>,
) -> impl IntoResponse {
    let symbol = q.symbol.trim();
    if symbol.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "symbol required"})),
        )
            .into_response();
    }
    if !watchlist::is_watchlist_symbol(symbol) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "symbol not in watchlist"})),
        )
            .into_response();
    }
    let points = crate::today_cache::get_today(state.redis.as_ref(), symbol).await;
    let data: Vec<serde_json::Value> = points
        .into_iter()
        .map(|(t, price)| serde_json::json!({ "t": t, "price": price }))
        .collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "symbol": symbol, "data": data })),
    )
        .into_response()
}

async fn history_handler(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> impl IntoResponse {
    let symbol = q.symbol.trim();
    if symbol.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "symbol required"})),
        )
            .into_response();
    }
    if !watchlist::is_watchlist_symbol(symbol) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "symbol not in watchlist"})),
        )
            .into_response();
    }
    let conid = {
        let g = state.watchlist_cache.read().await;
        match watchlist::get_conid_with_cache(symbol, &g) {
            Some(c) => c,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "unknown conid for symbol"})),
                )
                    .into_response()
            }
        }
    };
    let client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "IBKR not configured"})),
            )
                .into_response()
        }
    };
    let days = q.days.unwrap_or(7).min(365);
    // IBKR CP history: use week/month/year strings so the span matches the UI (7d→1w, 30d→1m, …).
    let period = match days {
        7 => "1w".to_string(),
        30 => "1m".to_string(),
        90 => "3m".to_string(),
        365 => "1y".to_string(),
        n => format!("{n}d"),
    };
    match client.fetch_history(conid, &period, "1d").await {
        Ok(bars) => {
            let bar_count = bars.len();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "symbol": symbol,
                    "data": bars,
                    "provider": "IBKR",
                    "period": period,
                    "bars": bar_count,
                })),
            )
                .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!("IBKR history failed for {}: {}", symbol, msg);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "symbol": symbol, "data": [], "provider": "IBKR", "error": msg })),
            )
                .into_response()
        }
    }
}

#[derive(serde::Deserialize)]
struct DailySignalsQuery {
    /// Cross-sectional top/bottom fraction (default 0.2).
    #[serde(default = "default_signal_quantile")]
    quantile: f64,
    /// If false, long-only (no short leg). Default: true unless `DAILY_STRATEGY_SHORT_ENABLED=0`.
    #[serde(default)]
    short: Option<bool>,
    /// Include simple long/short backtest on aligned history (default false).
    #[serde(default)]
    backtest: Option<bool>,
}

#[derive(serde::Deserialize)]
struct DailySignalsResearchQuery {
    #[serde(default = "default_signal_quantile")]
    quantile: f64,
    #[serde(default)]
    short: Option<bool>,
    /// Rolling train window size in trading days (default 120).
    #[serde(default = "default_train_days")]
    train_days: usize,
    /// Rolling test window size in trading days (default 20).
    #[serde(default = "default_test_days")]
    test_days: usize,
    /// Number of latest rows to include in response sample (default 300).
    #[serde(default = "default_sample_rows")]
    sample_rows: usize,
}

#[derive(serde::Deserialize)]
struct DailySignalsModelQuery {
    #[serde(default = "default_signal_quantile")]
    quantile: f64,
    #[serde(default)]
    short: Option<bool>,
    /// Rolling train window used for model fit (default 120).
    #[serde(default = "default_train_days")]
    train_days: usize,
}

fn default_signal_quantile() -> f64 {
    daily_strategy::DEFAULT_QUANTILE
}
fn default_train_days() -> usize {
    120
}
fn default_test_days() -> usize {
    20
}
fn default_sample_rows() -> usize {
    300
}

async fn daily_signals_handler(
    State(state): State<AppState>,
    Query(q): Query<DailySignalsQuery>,
) -> impl IntoResponse {
    let client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "IBKR not configured"})),
            )
                .into_response();
        }
    };

    let quantile = q.quantile.clamp(0.05, 0.45);
    let short_enabled = q.short.unwrap_or_else(|| {
        std::env::var("DAILY_STRATEGY_SHORT_ENABLED")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
    });
    let include_backtest = q.backtest.unwrap_or(false);

    let pairs: Vec<(String, u64)> = {
        let g = state.watchlist_cache.read().await;
        watchlist::DEFAULT_SYMBOLS
            .iter()
            .filter_map(|s| {
                let sym = s.to_string();
                watchlist::get_conid_with_cache(s, &g).map(|c| (sym, c))
            })
            .collect()
    };

    if pairs.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "need at least 3 watchlist symbols with conids"})),
        )
            .into_response();
    }

    let resp = daily_strategy::compute_daily_signals(
        client,
        &pairs,
        quantile,
        short_enabled,
        include_backtest,
    )
    .await;
    if !resp.data_available {
        tracing::info!(
            "daily-signals: no data (reason={:?}, symbols_with_history={}, overlapping_days={})",
            resp.reason,
            resp.symbols_with_history,
            resp.overlapping_days
        );
    }
    (StatusCode::OK, Json(resp)).into_response()
}

async fn daily_signals_research_handler(
    State(state): State<AppState>,
    Query(q): Query<DailySignalsResearchQuery>,
) -> impl IntoResponse {
    let client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "IBKR not configured"})),
            )
                .into_response();
        }
    };

    let quantile = q.quantile.clamp(0.05, 0.45);
    let short_enabled = q.short.unwrap_or_else(|| {
        std::env::var("DAILY_STRATEGY_SHORT_ENABLED")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
    });

    let pairs: Vec<(String, u64)> = {
        let g = state.watchlist_cache.read().await;
        watchlist::DEFAULT_SYMBOLS
            .iter()
            .filter_map(|s| watchlist::get_conid_with_cache(s, &g).map(|c| (s.to_string(), c)))
            .collect()
    };
    if pairs.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "need at least 3 watchlist symbols with conids"})),
        )
            .into_response();
    }

    let resp = daily_strategy::compute_phase2_research(
        client,
        &pairs,
        quantile,
        short_enabled,
        q.train_days,
        q.test_days,
        q.sample_rows.min(50000),
    )
    .await;
    (StatusCode::OK, Json(resp)).into_response()
}

async fn daily_signals_model_handler(
    State(state): State<AppState>,
    Query(q): Query<DailySignalsModelQuery>,
) -> impl IntoResponse {
    let client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "IBKR not configured"})),
            )
                .into_response();
        }
    };
    let quantile = q.quantile.clamp(0.05, 0.45);
    let short_enabled = q.short.unwrap_or_else(|| {
        std::env::var("DAILY_STRATEGY_SHORT_ENABLED")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
    });

    let pairs: Vec<(String, u64)> = {
        let g = state.watchlist_cache.read().await;
        watchlist::DEFAULT_SYMBOLS
            .iter()
            .filter_map(|s| watchlist::get_conid_with_cache(s, &g).map(|c| (s.to_string(), c)))
            .collect()
    };
    if pairs.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "need at least 3 watchlist symbols with conids"})),
        )
            .into_response();
    }

    let resp = daily_strategy::compute_phase2_model_signals(
        client,
        &pairs,
        quantile,
        short_enabled,
        q.train_days.max(60),
    )
    .await;
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(serde::Deserialize)]
struct DebugSnapshotQuery {
    /// Comma-separated list of symbols, e.g. `GOOGL,META`.
    /// If omitted, we debug all symbols that currently have `price = null`.
    symbols: Option<String>,
    /// Optional comma-separated snapshot field list (IBKR `/iserver/marketdata/snapshot`).
    /// If omitted, we omit `fields=` to let IBKR use its default.
    fields: Option<String>,
}

async fn debug_snapshot_handler(
    State(state): State<AppState>,
    Query(q): Query<DebugSnapshotQuery>,
) -> impl IntoResponse {
    let ibkr_client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "IBKR not configured"})),
            )
                .into_response();
        }
    };
    let cfg = match state.config.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "service config missing"})),
            )
                .into_response();
        }
    };

    let snapshot_symbols: Vec<String> = {
        let g = state.watchlist_cache.read().await;
        let from_query = q
            .symbols
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_ascii_uppercase())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !from_query.is_empty() {
            from_query
        } else {
            g.quotes
                .iter()
                .filter_map(|(sym, q)| if q.price.is_none() { Some(sym.clone()) } else { None })
                .collect()
        }
    }
    .into_iter()
    .filter(|s| watchlist::is_watchlist_symbol(s))
    .collect();

    if snapshot_symbols.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No valid symbols to debug"})),
        )
            .into_response();
    }

    let resolved_pairs: Vec<(String, u64)> = {
        let g = state.watchlist_cache.read().await;
        snapshot_symbols
            .iter()
            .filter_map(|sym| watchlist::get_conid_with_cache(sym, &g).map(|c| (sym.clone(), c)))
            .collect()
    };

    if resolved_pairs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Could not resolve any conids for requested symbols"})),
        )
            .into_response();
    }

    let conids: Vec<u64> = resolved_pairs.iter().map(|(_, c)| *c).collect();

    let raw = match watchlist::fetch_snapshot_raw_for_conids(
        ibkr_client.as_ref(),
        &cfg.ibkr_api_base_url,
        &conids,
        q.fields.as_deref(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": e})),
            )
                .into_response();
        }
    };

    let conid_to_symbol: Vec<(u64, String)> = resolved_pairs
        .iter()
        .map(|(s, c)| (*c, s.clone()))
        .collect();

    let parse_f64 = |item: &serde_json::Value, key: &str| -> Option<f64> {
        let parse_ibkr_number = |raw: &str| -> Option<f64> {
            let cleaned = raw.trim().replace(',', "");
            if let Ok(v) = cleaned.parse::<f64>() {
                return Some(v);
            }
            let trimmed = cleaned
                .trim_start_matches(|c: char| {
                    !(c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
                })
                .to_string();
            trimmed.parse::<f64>().ok()
        };
        item.get(key)
            .and_then(|v| v.as_f64())
            .or_else(|| item.get(key).and_then(|v| v.as_str()).and_then(parse_ibkr_number))
    };

    // Clone so we don't move `raw` (used later in the response).
    let snapshot_items = match &raw {
        serde_json::Value::Array(arr) => arr.clone(),
        other => vec![other.clone()],
    };

    let computed: Vec<serde_json::Value> = snapshot_items
        .iter()
        .filter_map(|item| {
            let conid = item
                .get("conid")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("conid").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))?;
            let symbol = conid_to_symbol
                .iter()
                .find(|(c, _)| *c == conid)
                .map(|(_, s)| s.clone())
                .unwrap_or_else(|| item.get("55").and_then(|v| v.as_str()).unwrap_or("?").to_string());

            let last = parse_f64(item, "31");
            let bid = parse_f64(item, "84");
            let ask = parse_f64(item, "86");
            let best = last
                .filter(|p| p.is_finite() && *p > 0.0)
                .or_else(|| match (bid, ask) {
                    (Some(b), Some(a)) => Some((b + a) / 2.0),
                    (Some(b), None) => Some(b),
                    (None, Some(a)) => Some(a),
                    _ => None,
                });

            Some(serde_json::json!({
                "symbol": symbol,
                "conid": conid,
                "fields": { "31_last": last, "84_bid": bid, "86_ask": ask },
                "computed_best_price": best,
            }))
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "requested_symbols": snapshot_symbols,
            "resolved": resolved_pairs.iter().map(|(s,c)| serde_json::json!({"symbol": s, "conid": c})).collect::<Vec<_>>(),
            "computed_from_fields": computed,
            "raw": raw,
        })),
    )
        .into_response()
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let ibkr_oauth_configured = state
        .config
        .as_ref()
        .map(|c| c.ibkr_oauth_configured())
        .unwrap_or(false);
    Json(serde_json::json!({
        "service": "stock-service",
        "version": env!("CARGO_PKG_VERSION"),
        "scope": "SSO verification and IBKR integration",
        "ibkr_oauth_configured": ibkr_oauth_configured
    }))
}

async fn ibkr_verify(State(state): State<AppState>) -> impl IntoResponse {
    let client = match state.ibkr_client.as_ref() {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "IBKR OAuth not configured"
                })),
            )
                .into_response()
        }
    };
    match client.get_live_session_token().await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "message": "IBKR authentication verified (LST obtained)" })),
        )
            .into_response(),
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!("IBKR verify failed: {}", msg);
            // IBKR 401 (invalid consumer, wrong realm, etc.) → return 401 so UI shows auth error, not "bad gateway"
            let is_unauthorized = msg.contains("401")
                || msg.contains("Unauthorized")
                || msg.contains("invalid consumer");
            let status = if is_unauthorized {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_GATEWAY
            };
            let hint = if is_unauthorized {
                "Check consumer key, realm (test_realm for TESTCONS, limited_poa for production), and that the key is active after midnight in the portal region."
            } else {
                ""
            };
            (
                status,
                Json(serde_json::json!({
                    "ok": false,
                    "error": msg,
                    "hint": hint
                })),
            )
                .into_response()
        }
    }
}

async fn me(
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
    display_name: Option<axum::extract::Extension<UserDisplayName>>,
    email: Option<axum::extract::Extension<UserEmail>>,
) -> Json<serde_json::Value> {
    let resolved_email = email.and_then(|axum::extract::Extension(v)| v.0);
    let resolved_display_name = display_name
        .map(|axum::extract::Extension(v)| v.0)
        .filter(|v| !v.trim().is_empty())
        .filter(|v| v != &user_id.0)
        .filter(|v| !looks_like_user_id(v))
        .or_else(|| {
            resolved_email
                .as_deref()
                .and_then(|v| v.split('@').next())
                .map(String::from)
        });
    Json(serde_json::json!({
        "user_id": user_id.0,
        "display_name": resolved_display_name,
        "email": resolved_email
    }))
}

#[derive(serde::Deserialize)]
struct SetGatewayCookieBody {
    cookie: String,
}

/// POST /api/v1/ibkr/gateway-cookie: set Gateway session cookie from authenticated client (e.g. after logging in at /ibkr-gateway/).
/// Requires IBKR_GATEWAY_SESSION_COOKIE_FILE to be set and writable.
async fn set_gateway_cookie(
    State(state): State<AppState>,
    Json(body): Json<SetGatewayCookieBody>,
) -> impl IntoResponse {
    let cookie = body.cookie.trim();
    if cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"cookie required"})));
    }
    let path = match &state.config {
        Some(cfg) => cfg.ibkr_gateway_session_cookie_file.as_deref(),
        None => None,
    };
    let path = match path {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"IBKR_GATEWAY_SESSION_COOKIE_FILE not configured"})),
            );
        }
    };
    if let Err(e) = tokio::fs::write(path, cookie).await {
        tracing::warn!("write gateway cookie file: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":"failed to write cookie file"})),
        );
    }
    (StatusCode::OK, Json(serde_json::json!({"ok":true})))
}
