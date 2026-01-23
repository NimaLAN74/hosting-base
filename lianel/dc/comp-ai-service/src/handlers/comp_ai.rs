use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use crate::config::AppConfig;
use crate::models::{CompAIRequest, CompAIResponse};


#[utoipa::path(
    post,
    path = "/api/v1/process",
    tag = "comp-ai",
    request_body = CompAIRequest,
    responses(
        (status = 200, description = "Request processed successfully", body = CompAIResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn process_request(
    State(config): State<Arc<AppConfig>>,
    Json(request): Json<CompAIRequest>,
) -> Result<Json<CompAIResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start_time = std::time::Instant::now();
    
    // TODO: Implement actual AI processing logic
    // For now, return a mock response
    let response = CompAIResponse {
        response: format!("Mock response to: {}", request.prompt),
        model_used: "mock-model".to_string(),
        tokens_used: Some(100),
        processing_time_ms: start_time.elapsed().as_millis() as u64,
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/history",
    tag = "comp-ai",
    params(
        ("limit" = Option<u32>, Query, description = "Limit (default: 100)"),
        ("offset" = Option<u32>, Query, description = "Offset (default: 0)"),
    ),
    responses(
        (status = 200, description = "Request history retrieved successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_request_history(
    State(_config): State<Arc<AppConfig>>,
    Query(params): Query<RequestHistoryQueryParams>,
) -> Result<Json<RequestHistoryResponse>, (StatusCode, Json<serde_json::Value>)> {
    // TODO: Implement database query to get request history
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    Ok(Json(RequestHistoryResponse {
        data: vec![],
        total: 0,
        limit,
        offset,
    }))
}
