use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub country_code: String,
    pub country_name: String,
    pub eu_member: bool,
    pub record_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub product_code: String,
    pub product_name: String,
    pub record_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Flow {
    pub flow_code: String,
    pub flow_name: String,
    pub record_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountriesResponse {
    pub countries: Vec<Country>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductsResponse {
    pub products: Vec<Product>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowsResponse {
    pub flows: Vec<Flow>,
}

