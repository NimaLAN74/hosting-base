use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompAIRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompAIResponse {
    pub response: String,
    pub model_used: String,
    pub tokens_used: Option<u32>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestHistory {
    pub id: i64,
    pub user_id: String,
    pub prompt: String,
    pub response: Option<String>,
    pub model_used: Option<String>,
    pub tokens_used: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestHistoryResponse {
    pub data: Vec<RequestHistory>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestHistoryQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
