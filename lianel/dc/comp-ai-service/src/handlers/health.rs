use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use utoipa::OpenApi;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "comp-ai-service",
        "version": "1.0.0"
    })))
}
