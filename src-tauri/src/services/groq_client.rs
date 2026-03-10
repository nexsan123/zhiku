use serde::{Deserialize, Serialize};

use crate::errors::AppError;

/// Groq API endpoint (OpenAI-compatible).
const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Default model for Groq free tier (14400 req/day).
const GROQ_MODEL: &str = "llama-3.1-8b-instant";

/// Timeout for Groq requests.
const TIMEOUT_SECS: u64 = 30;

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

/// Send a chat completion request to Groq API.
///
/// # Arguments
/// * `prompt` - User message content
/// * `api_key` - Groq API key (from tauri-plugin-store)
///
/// # Returns
/// The assistant's response text.
/// If `api_key` is empty, returns `Ok(String::new())` with a log warning (graceful degradation).
pub async fn chat_completion(prompt: &str, api_key: &str) -> Result<String, AppError> {
    if api_key.is_empty() {
        log::warn!("Groq API key not configured — skipping request");
        return Ok(String::new());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let request_body = ChatCompletionRequest {
        model: GROQ_MODEL,
        messages: vec![ChatMessage {
            role: "user",
            content: prompt,
        }],
    };

    let response = client
        .post(GROQ_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Groq request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Groq API returned status {}",
            response.status()
        )));
    }

    let result: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| AppError::Parse(format!("Failed to parse Groq response: {}", e)))?;

    let content = result
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    Ok(content)
}
