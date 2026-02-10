//! Axum router and handlers. Exposed for integration tests (oneshot) and main binary.

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
use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub validator: Arc<KeycloakJwtValidator>,
}

/// Build the application router (used by main and by API integration tests).
pub fn create_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status));

    let protected = Router::new()
        .route("/api/v1/me", get(me))
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
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(String::from));
    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing authorization token"})),
            )
                .into_response()
        }
    };
    match state.validator.validate_bearer(&token).await {
        Ok(sub) => {
            req.extensions_mut().insert(UserId(sub));
            next.run(req).await
        }
        Err(e) => {
            tracing::debug!("JWT validation failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid or expired token"})),
            )
                .into_response()
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn status(State(state): State<AppState>) -> String {
    let scope = serde_json::json!({
        "service": "stock-monitoring",
        "version": env!("CARGO_PKG_VERSION"),
        "scope": "EU markets MVP"
    });
    if sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok()
    {
        serde_json::json!({
            "service": scope["service"],
            "version": scope["version"],
            "scope": scope["scope"],
            "database": "connected"
        })
        .to_string()
    } else {
        scope.to_string()
    }
}

async fn me(
    axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "user_id": user_id.0 }))
}
