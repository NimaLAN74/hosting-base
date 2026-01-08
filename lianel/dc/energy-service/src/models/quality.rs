use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityDimension {
    pub completeness: String,
    pub validity: String,
    pub consistency: String,
    pub accuracy: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityReport {
    pub check_timestamp: DateTime<Utc>,
    pub overall_status: String,
    pub quality_score: f64,
    pub dimensions: QualityDimension,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityResponse {
    pub latest_report: QualityReport,
}

