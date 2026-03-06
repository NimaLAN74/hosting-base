//! Stock service: minimal API for SSO verification and IBKR authentication. Health, status, /me, /api/v1/ibkr/verify.

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::auth::{KeycloakJwtValidator, UserId};
use crate::ibkr::IbkrOAuthClient;

#[derive(Clone)]
pub struct AppState {
    pub validator: Arc<KeycloakJwtValidator>,
    pub config: Option<Arc<crate::config::AppConfig>>,
    pub ibkr_client: Option<Arc<IbkrOAuthClient>>,
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
        .route("/api/v1/status", get(status));

    let protected = Router::new()
        .route("/api/v1/me", get(me))
        .route("/api/v1/ibkr/verify", get(ibkr_verify))
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
            tracing::warn!("IBKR verify failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "ok": false,
                    "error": e.to_string()
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
