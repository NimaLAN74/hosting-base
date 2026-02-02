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
use crate::frameworks;
use crate::inference;
use crate::models::{
    ChatMessage, CompAIRequest, CompAIResponse, FrameworkItemResponse, FrameworksListResponse,
    RequestHistoryQueryParams, RequestHistoryResponse,
};
use crate::auth::extract_user;
use crate::db::queries::{save_request, get_request_history as get_request_history_from_db};
use crate::response_cache::{self, CachedResponse};

pub(crate) type ResponseCache = Option<std::sync::Arc<moka::sync::Cache<String, CachedResponse>>>;


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
    State((config, pool, cache)): State<(Arc<AppConfig>, PgPool, ResponseCache)>,
    Json(request): Json<CompAIRequest>,
) -> Result<Json<CompAIResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Authenticate user
    let user = extract_user(&headers, config.clone())
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))))?;

    // Validate prompt
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Prompt is required and cannot be empty"})),
        ));
    }
    if config.comp_ai_max_prompt_len > 0 && prompt.len() > config.comp_ai_max_prompt_len {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Prompt too long",
                "max_length": config.comp_ai_max_prompt_len,
                "received": prompt.len()
            })),
        ));
    }

    // Validate framework (optional; if set must be in allowlist)
    let framework_id = request.framework.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(fid) = framework_id {
        if !frameworks::is_supported(fid) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Unsupported framework",
                    "framework": fid,
                    "supported": frameworks::list().iter().map(|f| f.id).collect::<Vec<_>>()
                })),
            ));
        }
    }
    let prompt_prefix = framework_id.and_then(frameworks::prompt_prefix);

    // Validate messages if present (for chat): roles user/assistant, non-empty content, total length
    let messages: Option<Vec<ChatMessage>> = request.messages.as_ref().map(|msgs| {
        msgs.iter()
            .filter_map(|m| {
                let role = m.role.trim().to_lowercase();
                let content = m.content.trim();
                if content.is_empty() || (!role.is_empty() && role != "user" && role != "assistant" && role != "system") {
                    return None;
                }
                Some(ChatMessage {
                    role: if role.is_empty() { "user".to_string() } else { role },
                    content: content.to_string(),
                })
            })
            .collect()
    });
    let use_chat = messages.as_ref().map_or(false, |m| !m.is_empty());
    let messages_for_key: Option<Vec<ChatMessage>> = if use_chat {
        let mut full = messages.clone().unwrap_or_default();
        full.push(ChatMessage { role: "user".to_string(), content: prompt.to_string() });
        Some(full)
    } else {
        None
    };

    // Cache key: for single-turn use (framework, prompt); for chat use (framework, full messages including new prompt)
    let cache_key = response_cache::build_cache_key(
        framework_id,
        prompt,
        messages_for_key.as_deref().filter(|_| use_chat),
    );
    if let Some(ref c) = cache {
        if let Some(entry) = c.get(&cache_key) {
            tracing::info!("Response cache hit for user {}", user.sub);
            return Ok(Json(CompAIResponse {
                response: entry.response,
                model_used: entry.model_used,
                tokens_used: entry.tokens_used,
                processing_time_ms: 0,
            }));
        }
    }

    let start_time = std::time::Instant::now();
    tracing::info!(
        "Processing request from user: {} ({}) {}",
        user.preferred_username.as_deref().unwrap_or("unknown"),
        user.sub,
        if use_chat { "(chat)" } else { "(single)" }
    );

    let (response_text, model_used, tokens_used) = if let (Some(ref url), Some(ref model)) =
        (&config.comp_ai_ollama_url, &config.comp_ai_ollama_model)
    {
        if use_chat {
            // Build (role, content) list: optional system from framework prefix, then history, then new user prompt
            let mut ollama_messages: Vec<(String, String)> = Vec::new();
            if let Some(prefix) = prompt_prefix {
                ollama_messages.push(("system".to_string(), prefix.to_string()));
            }
            for m in messages.as_deref().unwrap_or(&[]) {
                ollama_messages.push((m.role.clone(), m.content.clone()));
            }
            ollama_messages.push(("user".to_string(), prompt.to_string()));
            match inference::chat(
                url,
                model,
                &ollama_messages,
                config.comp_ai_max_tokens,
                config.comp_ai_temperature,
            )
            .await
            {
                Ok((text, count)) => (text, model.clone(), count),
                Err(e) => {
                    tracing::error!("Ollama chat failed: {}", e);
                    if config.comp_ai_ollama_fallback_to_mock {
                        let response_text = format!("Mock response to: {}", prompt);
                        (response_text, "mock-model (Ollama unavailable)".to_string(), Some(100))
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
            match inference::generate(
                url,
                model,
                prompt,
                config.comp_ai_max_tokens,
                config.comp_ai_temperature,
                prompt_prefix,
            )
            .await
            {
                Ok((text, count)) => (text, model.clone(), count),
                Err(e) => {
                    tracing::error!("Ollama inference failed: {}", e);
                    if config.comp_ai_ollama_fallback_to_mock {
                        let response_text = format!("Mock response to: {}", prompt);
                        (response_text, "mock-model (Ollama unavailable)".to_string(), Some(100))
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
        }
    } else {
        let response_text = format!("Mock response to: {}", prompt);
        (response_text, "mock-model".to_string(), Some(100))
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    if let Some(ref c) = cache {
        c.insert(
            cache_key,
            CachedResponse {
                response: response_text.clone(),
                model_used: model_used.clone(),
                tokens_used,
            },
        );
    }

    let response = CompAIResponse {
        response: response_text.clone(),
        model_used: model_used.clone(),
        tokens_used,
        processing_time_ms,
    };

    if let Err(e) = save_request(
        &pool,
        &user.sub,
        prompt,
        Some(&response_text),
        Some(&model_used),
        tokens_used.map(|t| t as i32),
        processing_time_ms as i64,
        "completed",
        None,
    )
    .await
    {
        tracing::error!("Failed to save request to database: {}", e);
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
    State((config, pool, _cache)): State<(Arc<AppConfig>, PgPool, ResponseCache)>,
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

#[utoipa::path(
    get,
    path = "/api/v1/frameworks",
    tag = "comp-ai",
    responses(
        (status = 200, description = "List of supported compliance frameworks", body = FrameworksListResponse),
    )
)]
pub async fn get_frameworks(State((_config, _pool, _cache)): State<(Arc<AppConfig>, PgPool, ResponseCache)>) -> Json<FrameworksListResponse> {
    let frameworks = frameworks::list()
        .iter()
        .map(|f| FrameworkItemResponse {
            id: f.id.to_string(),
            name: f.name.to_string(),
            description: f.description.to_string(),
        })
        .collect();
    Json(FrameworksListResponse { frameworks })
}
