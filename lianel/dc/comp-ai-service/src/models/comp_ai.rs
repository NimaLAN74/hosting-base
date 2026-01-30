use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

/// One message in a conversation (for multi-turn chat).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessage {
    pub role: String,   // "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompAIRequest {
    pub prompt: String,
    /// Optional compliance framework id (e.g. soc2, iso27001, gdpr). Use GET /api/v1/frameworks for list.
    pub framework: Option<String>,
    /// Optional conversation history. When present, backend uses Ollama /api/chat for multi-turn context. Chronological order; last item should be the new user message (or backend appends prompt).
    pub messages: Option<Vec<ChatMessage>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub model: Option<String>,
}

/// One framework in the list returned by GET /api/v1/frameworks.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FrameworkItemResponse {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FrameworksListResponse {
    pub frameworks: Vec<FrameworkItemResponse>,
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
