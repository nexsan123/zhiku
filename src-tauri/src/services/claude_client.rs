// Phase 3 deep analysis client — not yet wired into commands/poll_loop.
// Will be used for financial cycle reasoning, geopolitical analysis, etc.

use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::services::ai_config::ResolvedAiConfig;

/// Default Claude API endpoint (used when user has not configured a custom endpoint).
const DEFAULT_CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Default model for deep analysis (used when user has not configured a model name).
const DEFAULT_CLAUDE_MODEL: &str = "claude-sonnet-4-20250514";

/// Anthropic API version header.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Timeout for Claude requests (longer for deep analysis).
const TIMEOUT_SECS: u64 = 60;

/// Max tokens for Claude response.
const MAX_TOKENS: u32 = 4096;

/// Claude API message.
#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Claude API request body.
#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
}

/// Claude API response.
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

/// Send an analysis request to Claude API using resolved config.
///
/// Reads model name and endpoint URL from the user's config. If not set,
/// falls back to built-in defaults (claude-sonnet-4-20250514 + api.anthropic.com).
///
/// # Arguments
/// * `prompt` - User message content
/// * `system` - Optional system prompt for context setting
/// * `config` - Resolved AI config (api_key, model_name, endpoint_url from settings)
///
/// # Returns
/// The assistant's response text.
/// If `api_key` is empty, returns `Ok(String::new())` with a log warning (graceful degradation).
pub async fn analyze(
    prompt: &str,
    system: Option<&str>,
    config: &ResolvedAiConfig,
) -> Result<String, AppError> {
    if config.api_key.is_empty() {
        log::warn!("Claude API key not configured — skipping request");
        return Ok(String::new());
    }

    let model = if config.model_name.is_empty() {
        DEFAULT_CLAUDE_MODEL
    } else {
        &config.model_name
    };

    let endpoint = if config.endpoint_url.is_empty() {
        DEFAULT_CLAUDE_API_URL
    } else {
        &config.endpoint_url
    };

    log::info!("Claude request: model={}, endpoint={}", model, endpoint);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let request_body = ClaudeRequest {
        model,
        max_tokens: MAX_TOKENS,
        messages: vec![ClaudeMessage {
            role: "user",
            content: prompt,
        }],
        system,
    };

    let response = client
        .post(endpoint)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Claude request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Claude API returned status {}",
            response.status()
        )));
    }

    let result: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| AppError::Parse(format!("Failed to parse Claude response: {}", e)))?;

    let text = result
        .content
        .into_iter()
        .filter_map(|block| block.text)
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}


