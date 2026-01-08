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
    path = "/api/energy/annual",
    tag = "energy",
    params(
        ("country_code" = Option<String>, Query, description = "Country code filter"),
        ("year" = Option<i32>, Query, description = "Year filter"),
        ("product_code" = Option<String>, Query, description = "Product code filter"),
        ("flow_code" = Option<String>, Query, description = "Flow code filter"),
        ("source_table" = Option<String>, Query, description = "Source table filter"),
        ("limit" = Option<u32>, Query, description = "Limit (default: 1000)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Energy data retrieved successfully", body = EnergyResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_energy_annual(
    State(pool): State<PgPool>,
    Query(params): Query<EnergyQueryParams>,
) -> Result<Json<EnergyResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000); // Max 10k records
    let offset = params.offset.unwrap_or(0);

    match queries::get_energy_records(
        &pool,
        params.country_code.as_deref(),
        params.year,
        params.product_code.as_deref(),
        params.flow_code.as_deref(),
        params.source_table.as_deref(),
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(EnergyResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query energy data",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/energy/annual/by-country/{country_code}",
    tag = "energy",
    params(
        ("country_code" = String, Path, description = "Country code"),
    ),
    responses(
        (status = 200, description = "Energy data retrieved successfully", body = EnergyResponse),
    )
)]
pub async fn get_energy_by_country(
    State(pool): State<PgPool>,
    Path(country_code): Path<String>,
    Query(params): Query<EnergyQueryParams>,
) -> Result<Json<EnergyResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match queries::get_energy_records(
        &pool,
        Some(&country_code),
        params.year,
        params.product_code.as_deref(),
        params.flow_code.as_deref(),
        params.source_table.as_deref(),
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(EnergyResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query energy data",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/energy/annual/by-year/{year}",
    tag = "energy",
    params(
        ("year" = i32, Path, description = "Year"),
    ),
    responses(
        (status = 200, description = "Energy data retrieved successfully", body = EnergyResponse),
    )
)]
pub async fn get_energy_by_year(
    State(pool): State<PgPool>,
    Path(year): Path<i32>,
    Query(params): Query<EnergyQueryParams>,
) -> Result<Json<EnergyResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(1000).min(10000);
    let offset = params.offset.unwrap_or(0);

    match queries::get_energy_records(
        &pool,
        params.country_code.as_deref(),
        Some(year),
        params.product_code.as_deref(),
        params.flow_code.as_deref(),
        params.source_table.as_deref(),
        limit,
        offset,
    ).await {
        Ok((records, total)) => Ok(Json(EnergyResponse {
            data: records,
            total,
            limit,
            offset,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query energy data",
                "details": e.to_string()
            })),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/energy/annual/summary",
    tag = "energy",
    params(
        ("country_code" = Option<String>, Query, description = "Country code filter"),
        ("year" = Option<i32>, Query, description = "Year filter"),
        ("group_by" = Option<String>, Query, description = "Group by: country, year, product, flow (default: country)"),
    ),
    responses(
        (status = 200, description = "Summary retrieved successfully", body = EnergySummaryResponse),
    )
)]
pub async fn get_energy_summary(
    State(pool): State<PgPool>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<EnergySummaryResponse>, (StatusCode, Json<serde_json::Value>)> {
    let country_code = params.get("country_code")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let year = params.get("year")
        .and_then(|v| v.as_i64())
        .map(|y| y as i32);
    let group_by = params.get("group_by")
        .and_then(|v| v.as_str())
        .unwrap_or("country");

    match queries::get_energy_summary(
        &pool,
        country_code.as_deref(),
        year,
        group_by,
    ).await {
        Ok(results) => {
            let summary: Vec<EnergySummary> = results
                .into_iter()
                .map(|(group, total_gwh, record_count)| EnergySummary {
                    group,
                    total_gwh,
                    record_count,
                })
                .collect();

            let total_records: i64 = summary.iter().map(|s| s.record_count).sum();

            Ok(Json(EnergySummaryResponse {
                summary,
                total_records,
            }))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to query summary",
                "details": e.to_string()
            })),
        )),
    }
}

