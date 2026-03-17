//! Unified OpenAI-compatible chat completion client.
//!
//! Works with: Groq, Ollama (/v1/), DeepSeek, Mistral, vLLM, LM Studio,
//! Together, Fireworks, OpenAI, and any provider implementing the
//! `/v1/chat/completions` endpoint.

use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::services::ai_config::ResolvedAiConfig;

/// Timeout for chat completion requests.
const TIMEOUT_SECS: u64 = 60;

/// OpenAI-compatible chat message.
#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// OpenAI-compatible chat completion request.
#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// OpenAI-compatible chat completion response.
#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

/// Default endpoints per provider (used when user has not configured a custom endpoint).
fn default_endpoint(provider: &str) -> &'static str {
    match provider {
        "groq" => "https://api.groq.com/openai/v1/chat/completions",
        "ollama" => "http://localhost:11434/v1/chat/completions",
        "deepseek" => "https://api.deepseek.com/v1/chat/completions",
        "mistral" => "https://api.mistral.ai/v1/chat/completions",
        "openai" => "https://api.openai.com/v1/chat/completions",
        _ => "http://localhost:11434/v1/chat/completions", // fallback to local
    }
}

/// Default model per provider (used when user has not configured a model name).
fn default_model(provider: &str) -> &'static str {
    match provider {
        "groq" => "llama-3.1-8b-instant",
        "ollama" => "llama3.1:8b",
        "deepseek" => "deepseek-chat",
        "mistral" => "mistral-small-latest",
        "openai" => "gpt-4o-mini",
        _ => "llama3.1:8b",
    }
}

/// Send a chat completion request to any OpenAI-compatible endpoint.
///
/// Supports system prompt (sent as a separate message with role="system").
///
/// # Arguments
/// * `prompt` - User message content
/// * `system` - Optional system prompt
/// * `config` - Resolved AI config (api_key, model_name, endpoint_url)
/// * `provider` - Provider name (for default endpoint/model lookup)
pub async fn chat_completion(
    prompt: &str,
    system: Option<&str>,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<String, AppError> {
    // Ollama doesn't need an API key; others do
    let needs_key = provider != "ollama";
    if needs_key && config.api_key.is_empty() {
        log::warn!("{} API key not configured — skipping request", provider);
        return Ok(String::new());
    }

    let model = if config.model_name.is_empty() {
        default_model(provider)
    } else {
        &config.model_name
    };

    let endpoint = if config.endpoint_url.is_empty() {
        default_endpoint(provider).to_string()
    } else {
        // If user gave a base URL (e.g. "https://api.groq.com"), append the path
        let url = config.endpoint_url.trim_end_matches('/');
        if url.ends_with("/chat/completions") {
            url.to_string()
        } else if url.ends_with("/v1") {
            format!("{}/chat/completions", url)
        } else {
            format!("{}/v1/chat/completions", url)
        }
    };

    log::info!("OpenAI-compat request: provider={}, model={}, endpoint={}", provider, model, endpoint);

    // Build messages: system (optional) + user
    let mut messages = Vec::new();
    if let Some(sys) = system {
        messages.push(ChatMessage {
            role: "system",
            content: sys,
        });
    }
    messages.push(ChatMessage {
        role: "user",
        content: prompt,
    });

    let request_body = ChatCompletionRequest {
        model,
        messages,
        max_tokens: Some(4096),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let mut req = client
        .post(&endpoint)
        .header("Content-Type", "application/json");

    // Add auth header if we have a key
    if !config.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = req
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("{} request failed: {}", provider, e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Network(format!(
            "{} API returned status {}: {}",
            provider,
            status,
            &body[..body.len().min(200)]
        )));
    }

    let result: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| AppError::Parse(format!("Failed to parse {} response: {}", provider, e)))?;

    let content = result
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    Ok(content)
}
