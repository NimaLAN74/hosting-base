//! Process-local response cache for Comp-AI (key = hash of request, value = response).
//! TTL and max capacity from config; disabled when COMP_AI_RESPONSE_CACHE_TTL_SECS=0.

use crate::models::ChatMessage;
use sha2::{Sha256, Digest};
use std::time::Duration;

/// Cached response (cloneable for returning from cache).
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub response: String,
    pub model_used: String,
    pub tokens_used: Option<u32>,
}

/// Build a cache key from framework + prompt or messages (canonical JSON then SHA-256 hex).
pub fn build_cache_key(
    framework_id: Option<&str>,
    prompt: &str,
    messages_opt: Option<&[ChatMessage]>,
) -> String {
    let canonical = match messages_opt {
        None => serde_json::json!({ "f": framework_id.unwrap_or(""), "p": prompt }).to_string(),
        Some(m) => serde_json::json!({ "f": framework_id.unwrap_or(""), "m": m }).to_string(),
    };
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Create a new moka cache with TTL and max capacity. Returns None if ttl_secs == 0.
pub fn create_cache(ttl_secs: u64, max_entries: u64) -> Option<moka::sync::Cache<String, CachedResponse>> {
    if ttl_secs == 0 || max_entries == 0 {
        return None;
    }
    let cache = moka::sync::Cache::builder()
        .time_to_live(Duration::from_secs(ttl_secs))
        .max_capacity(max_entries)
        .build();
    Some(cache)
}
