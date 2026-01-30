//! Local inference via Ollama HTTP API.
//! Used when COMP_AI_OLLAMA_URL and COMP_AI_OLLAMA_MODEL are set.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    num_predict: u32,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: Option<String>,
    eval_count: Option<u32>,
    #[allow(dead_code)]
    done: Option<bool>,
}

/// Call Ollama /api/generate. Returns (response_text, token_count).
/// If `prompt_prefix` is Some, it is prepended to the user prompt (e.g. compliance context).
pub async fn generate(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f64,
    prompt_prefix: Option<&str>,
) -> Result<(String, Option<u32>)> {
    let full_prompt = match prompt_prefix {
        Some(prefix) => format!("{}\n\nUser question: {}", prefix, prompt),
        None => prompt.to_string(),
    };
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = GenerateRequest {
        model: model.to_string(),
        prompt: full_prompt,
        stream: false,
        options: GenerateOptions {
            num_predict: max_tokens,
            temperature,
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("build reqwest client")?;

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Ollama generate request")?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Ollama returned {}: {}", status, text);
    }

    let parsed: GenerateResponse = res.json().await.context("parse Ollama response")?;
    let text = parsed
        .response
        .unwrap_or_else(|| "".to_string())
        .trim()
        .to_string();
    Ok((text, parsed.eval_count))
}

// --- Chat (multi-turn) ---

#[derive(Debug, Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    role: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Option<ChatResponseMessage>,
    eval_count: Option<u32>,
    #[allow(dead_code)]
    done: Option<bool>,
}

/// Call Ollama /api/chat with full message history. Returns (response_text, token_count).
/// `messages` must be chronological; system/framework context can be first as a "system" or "user" message.
pub async fn chat(
    base_url: &str,
    model: &str,
    messages: &[(String, String)], // (role, content)
    max_tokens: u32,
    temperature: f64,
) -> Result<(String, Option<u32>)> {
    let ollama_messages: Vec<OllamaChatMessage> = messages
        .iter()
        .map(|(role, content)| OllamaChatMessage {
            role: role.clone(),
            content: content.clone(),
        })
        .collect();
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let body = ChatRequest {
        model: model.to_string(),
        messages: ollama_messages,
        stream: false,
        options: GenerateOptions {
            num_predict: max_tokens,
            temperature,
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("build reqwest client")?;

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Ollama chat request")?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Ollama returned {}: {}", status, text);
    }

    let parsed: ChatResponse = res.json().await.context("parse Ollama chat response")?;
    let text = parsed
        .message
        .and_then(|m| m.content)
        .unwrap_or_else(|| "".to_string())
        .trim()
        .to_string();
    Ok((text, parsed.eval_count))
}
