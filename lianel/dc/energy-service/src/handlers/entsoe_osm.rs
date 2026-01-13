use axum::{
    extract::{Query, State},
    response::Json,
    http::StatusCode,
};
use sqlx::PgPool;
use crate::models::*;
use crate::db::*;

#[utoipa::path(
    get,
    path = "/api/v1/electricity/timeseries",
    tag = "electricity",
    params(
        ("country_code" = Option<String>, Query, description = "Country code filter"),
        ("start_date" = Option<String>, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = Option<String>, Query, description = "End date (YYYY-MM-DD)"),
        ("production_type" = Option<String>, Query, description = "Production type filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Electricity timeseries data retrieved successfully", body = ElectricityTimeseriesResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_electricity_timeseries(
    State(pool): State<PgPool>,
    Query(params): Query<ElectricityTimeseriesQueryParams>,
) -> Result<Json<ElectricityTimeseriesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match get_electricity_timeseries_records(
        &pool,
        params.country_code.as_deref(),
        params.start_date.as_deref(),
        params.end_date.as_deref(),
        params.production_type.as_deref(),
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(ElectricityTimeseriesResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query electricity timeseries data",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/geo/features",
    tag = "geo",
    params(
        ("region_id" = Option<String>, Query, description = "Region ID (NUTS code) filter"),
        ("feature_name" = Option<String>, Query, description = "Feature name filter"),
        ("snapshot_year" = Option<i32>, Query, description = "Snapshot year filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Geo features retrieved successfully", body = GeoFeatureResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_geo_features(
    State(pool): State<PgPool>,
    Query(params): Query<GeoFeatureQueryParams>,
) -> Result<Json<GeoFeatureResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match get_geo_feature_records(
        &pool,
        params.region_id.as_deref(),
        params.feature_name.as_deref(),
        params.snapshot_year,
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(GeoFeatureResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query geo features",
                "details": e.to_string()
            })),
        )),
    }
}
