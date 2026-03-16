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
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/watchlist", get(watchlist_handler))
        .route("/api/v1/history", get(history_handler))
        .route("/api/v1/today", get(today_handler));

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
    if watchlist::get_conid_for_symbol(symbol).is_none() {
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
    let conid = match watchlist::get_conid_for_symbol(symbol) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "symbol not in watchlist"})),
            )
                .into_response()
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
    let period = format!("{}d", days);
    match client.fetch_history(conid, &period, "1d").await {
        Ok(bars) => (
            StatusCode::OK,
            Json(serde_json::json!({ "symbol": symbol, "data": bars, "provider": "IBKR" })),
        )
            .into_response(),
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
