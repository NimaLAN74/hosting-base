use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use sqlx::PgPool;
use crate::models::*;

#[utoipa::path(
    get,
    path = "/api/info",
    tag = "info",
    responses(
        (status = 200, description = "Service information", body = serde_json::Value),
    )
)]
pub async fn get_service_info(
    State(pool): State<PgPool>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match crate::db::queries::get_database_stats(&pool).await {
        Ok((total_records, countries, years, tables)) => {
            Ok(Json(serde_json::json!({
                "service": "lianel-energy-service",
                "version": "1.0.0",
                "database": {
                    "connected": true,
                    "total_records": total_records,
                    "countries": countries,
                    "years": years,
                    "tables": tables
                }
            })))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to get service info",
                "details": e.to_string()
            })),
        )),
    }
}

