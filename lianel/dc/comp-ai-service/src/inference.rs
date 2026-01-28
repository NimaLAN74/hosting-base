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
pub async fn generate(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f64,
) -> Result<(String, Option<u32>)> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
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
