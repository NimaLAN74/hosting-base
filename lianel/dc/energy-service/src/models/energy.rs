use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct EnergyRecord {
    pub id: i64,
    pub country_code: String,
    pub country_name: Option<String>,
    pub year: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub flow_code: Option<String>,
    pub flow_name: Option<String>,
    pub value_gwh: f64,
    pub unit: Option<String>,
    pub source_table: String,
    pub ingestion_timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnergyQueryParams {
    pub country_code: Option<String>,
    pub year: Option<i32>,
    pub product_code: Option<String>,
    pub flow_code: Option<String>,
    pub source_table: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnergyResponse {
    pub data: Vec<EnergyRecord>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnergySummary {
    pub group: String,
    pub total_gwh: f64,
    pub record_count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnergySummaryResponse {
    pub summary: Vec<EnergySummary>,
    pub total_records: i64,
}

