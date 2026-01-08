use axum::response::Json;
use serde_json::json;
use sqlx::PgPool;

#[utoipa::path(
    get,
    path = "/api/energy/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value)
    )
)]
pub async fn health_check(pool: axum::extract::State<PgPool>) -> Json<serde_json::Value> {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").execute(&*pool).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    Json(json!({
        "status": "ok",
        "service": "lianel-energy-service",
        "database": db_status,
        "version": "1.0.0"
    }))
}

