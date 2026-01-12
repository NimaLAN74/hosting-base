use axum::{
    extract::{Query, State, Path},
    response::Json,
    http::StatusCode,
};
use sqlx::PgPool;
use crate::models::*;
use crate::db::queries;

#[utoipa::path(
    get,
    path = "/api/v1/datasets/forecasting",
    tag = "ml-datasets",
    params(
        ("cntr_code" = Option<String>, Query, description = "Country code filter"),
        ("year" = Option<i32>, Query, description = "Year filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Forecasting dataset retrieved successfully", body = ForecastingResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_forecasting_dataset(
    State(pool): State<PgPool>,
    Query(params): Query<MLDatasetQueryParams>,
) -> Result<Json<ForecastingResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match queries::get_forecasting_records(
        &pool,
        params.cntr_code.as_deref(),
        params.year,
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(ForecastingResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query forecasting dataset",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/datasets/clustering",
    tag = "ml-datasets",
    params(
        ("cntr_code" = Option<String>, Query, description = "Country code filter"),
        ("year" = Option<i32>, Query, description = "Year filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Clustering dataset retrieved successfully", body = ClusteringResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_clustering_dataset(
    State(pool): State<PgPool>,
    Query(params): Query<MLDatasetQueryParams>,
) -> Result<Json<ClusteringResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match queries::get_clustering_records(
        &pool,
        params.cntr_code.as_deref(),
        params.year,
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(ClusteringResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query clustering dataset",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/datasets/geo-enrichment",
    tag = "ml-datasets",
    params(
        ("cntr_code" = Option<String>, Query, description = "Country code filter"),
        ("year" = Option<i32>, Query, description = "Year filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Geo-enrichment dataset retrieved successfully", body = GeoEnrichmentResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_geo_enrichment_dataset(
    State(pool): State<PgPool>,
    Query(params): Query<MLDatasetQueryParams>,
) -> Result<Json<GeoEnrichmentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match queries::get_geo_enrichment_records(
        &pool,
        params.cntr_code.as_deref(),
        params.year,
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(GeoEnrichmentResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query geo-enrichment dataset",
                "details": e.to_string()
            })),
        )),
    }
}
