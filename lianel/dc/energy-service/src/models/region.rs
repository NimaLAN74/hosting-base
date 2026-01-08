use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub nuts_code: String,
    pub nuts_name: String,
    pub nuts_level: i32,
    pub country_code: String,
    pub area_km2: Option<f64>,
    pub geometry: Option<String>,
    pub energy_total_gwh: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegionsResponse {
    pub regions: Vec<Region>,
}

