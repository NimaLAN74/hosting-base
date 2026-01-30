use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx::PgPool;
use utoipa::ToSchema;
use crate::config::AppConfig;
use crate::inference;
use crate::models::{CompAIRequest, CompAIResponse, RequestHistoryQueryParams, RequestHistoryResponse};
use crate::auth::{AuthenticatedUser, extract_user};
use crate::db::queries::{save_request, get_request_history as get_request_history_from_db};


#[utoipa::path(
    post,
    path = "/api/v1/process",
    tag = "comp-ai",
    request_body = CompAIRequest,
    responses(
        (status = 200, description = "Request processed successfully", body = CompAIResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too many requests (rate limit)"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn process_request(
    headers: HeaderMap,
    State((config, pool)): State<(Arc<AppConfig>, PgPool)>,
    Json(request): Json<CompAIRequest>,
) -> Result<Json<CompAIResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Authenticate user
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;
    let start_time = std::time::Instant::now();
    
    tracing::info!(
        "Processing request from user: {} ({})",
        user.preferred_username.as_deref().unwrap_or("unknown"),
        user.sub
    );

    let (response_text, model_used, tokens_used) = if let (Some(ref url), Some(ref model)) =
        (&config.comp_ai_ollama_url, &config.comp_ai_ollama_model)
    {
        match inference::generate(
            url,
            model,
            &request.prompt,
            config.comp_ai_max_tokens,
            config.comp_ai_temperature,
        )
        .await
        {
            Ok((text, count)) => (text, model.clone(), count),
            Err(e) => {
                tracing::error!("Ollama inference failed: {}", e);
                if config.comp_ai_ollama_fallback_to_mock {
                    tracing::warn!("Falling back to mock (COMP_AI_OLLAMA_FALLBACK_TO_MOCK=true)");
                    let response_text = format!("Mock response to: {}", request.prompt);
                    let model_used = "mock-model (Ollama unavailable)".to_string();
                    let tokens_used = Some(100);
                    (response_text, model_used, tokens_used)
                } else {
                    return Err((
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(serde_json::json!({
                            "error": "Local model unavailable",
                            "detail": e.to_string()
                        })),
                    ));
                }
            }
        }
    } else {
        // No Ollama configured: mock response
        let response_text = format!("Mock response to: {}", request.prompt);
        let model_used = "mock-model".to_string();
        let tokens_used = Some(100);
        (response_text, model_used, tokens_used)
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;
    
    let response = CompAIResponse {
        response: response_text.clone(),
        model_used: model_used.clone(),
        tokens_used,
        processing_time_ms,
    };

    // Save request to database
    if let Err(e) = save_request(
        &pool,
        &user.sub,
        &request.prompt,
        Some(&response_text),
        Some(&model_used),
        tokens_used.map(|t| t as i32),
        processing_time_ms as i64,
        "completed",
        None,
    ).await {
        tracing::error!("Failed to save request to database: {}", e);
        // Continue anyway - don't fail the request if DB save fails
    }

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
        (status = 429, description = "Too many requests (rate limit)"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_request_history(
    headers: HeaderMap,
    State((config, pool)): State<(Arc<AppConfig>, PgPool)>,
    Query(params): Query<RequestHistoryQueryParams>,
) -> Result<Json<RequestHistoryResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Authenticate user
    let user = extract_user(&headers, config)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;
    tracing::info!(
        "Getting request history for user: {} ({})",
        user.preferred_username.as_deref().unwrap_or("unknown"),
        user.sub
    );
    
    let limit = params.limit.unwrap_or(100).min(1000); // Cap at 1000
    let offset = params.offset.unwrap_or(0);

    // Get request history from database
    match get_request_history_from_db(&pool, &user.sub, limit, offset).await {
        Ok((data, total)) => {
            Ok(Json(RequestHistoryResponse {
                data,
                total,
                limit,
                offset,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get request history: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to retrieve request history"})),
            ))
        }
    }
}
