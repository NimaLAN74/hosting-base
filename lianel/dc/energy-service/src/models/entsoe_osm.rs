use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ElectricityTimeseriesQueryParams {
    pub country_code: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub production_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ElectricityTimeseriesRecord {
    pub id: i64,
    pub timestamp_utc: chrono::NaiveDateTime,
    pub country_code: String,
    pub bidding_zone: Option<String>,
    pub production_type: Option<String>,
    pub load_mw: Option<f64>,
    pub generation_mw: Option<f64>,
    pub resolution: String,
    pub quality_flag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ElectricityTimeseriesResponse {
    pub data: Vec<ElectricityTimeseriesRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoFeatureQueryParams {
    pub region_id: Option<String>,
    pub feature_name: Option<String>,
    pub snapshot_year: Option<i32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoFeatureRecord {
    pub id: i64,
    pub region_id: String,
    pub feature_name: String,
    pub feature_value: f64,
    pub snapshot_year: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeoFeatureResponse {
    pub data: Vec<GeoFeatureRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}
