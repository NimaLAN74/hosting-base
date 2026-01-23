use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MLDatasetQueryParams {
    pub cntr_code: Option<String>,
    pub year: Option<i32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForecastingRecord {
    pub cntr_code: String,
    pub year: i32,
    pub total_energy_gwh: Option<f64>,
    pub renewable_energy_gwh: Option<f64>,
    pub fossil_energy_gwh: Option<f64>,
    pub pct_renewable: Option<f64>,
    pub pct_fossil: Option<f64>,
    pub yoy_change_total_energy_pct: Option<f64>,
    pub yoy_change_renewable_pct: Option<f64>,
    pub energy_density_gwh_per_km2: Option<f64>,
    pub area_km2: Option<f64>,
    pub year_index: Option<i32>,
    pub lag_1_year_total_energy_gwh: Option<f64>,
    pub lag_2_year_total_energy_gwh: Option<f64>,
    pub rolling_3y_mean_total_energy_gwh: Option<f64>,
    pub rolling_5y_mean_total_energy_gwh: Option<f64>,
    pub trend_3y_slope: Option<f64>,
    pub trend_5y_slope: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusteringRecord {
    pub cntr_code: String,
    pub year: i32,
    pub total_energy_gwh: Option<f64>,
    pub renewable_energy_gwh: Option<f64>,
    pub fossil_energy_gwh: Option<f64>,
    pub pct_renewable: Option<f64>,
    pub pct_fossil: Option<f64>,
    pub energy_density_gwh_per_km2: Option<f64>,
    pub area_km2: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoEnrichmentRecord {
    pub region_id: String,
    pub cntr_code: String,
    pub year: i32,
    pub total_energy_gwh: Option<f64>,
    pub renewable_energy_gwh: Option<f64>,
    pub fossil_energy_gwh: Option<f64>,
    pub pct_renewable: Option<f64>,
    pub energy_density_gwh_per_km2: Option<f64>,
    pub area_km2: Option<f64>,
    pub osm_feature_count: Option<i32>,
    pub power_plant_count: Option<i32>,
    pub industrial_area_km2: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MLDatasetResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

// Specific response types for OpenAPI
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForecastingResponse {
    pub data: Vec<ForecastingRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusteringResponse {
    pub data: Vec<ClusteringRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoEnrichmentResponse {
    pub data: Vec<GeoEnrichmentRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}
