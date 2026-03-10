use serde::{Deserialize, Serialize};

use crate::errors::AppError;

/// Ollama API base URL (local instance).
const OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Default timeout for Ollama requests (30 seconds for inference).
const TIMEOUT_SECS: u64 = 30;

/// Request body for Ollama /api/generate endpoint.
#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

/// Response from Ollama /api/generate endpoint.
#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

/// Check if Ollama is running and reachable.
///
/// Sends GET to /api/tags — a lightweight endpoint that lists available models.
/// Returns true if Ollama responds with 200, false on any error.
#[allow(dead_code)]
pub async fn check_health() -> Result<bool, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    match client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Generate a completion from Ollama.
///
/// # Arguments
/// * `prompt` - The prompt text to send
/// * `model` - Model name (e.g., "llama3.1:8b", "qwen2.5:14b")
///
/// # Returns
/// The generated text, or `AppError::Network` if Ollama is unreachable.
pub async fn generate_completion(prompt: &str, model: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let request_body = GenerateRequest {
        model,
        prompt,
        stream: false,
    };

    let response = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Ollama request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Ollama returned status {}",
            response.status()
        )));
    }

    let result: GenerateResponse = response
        .json()
        .await
        .map_err(|e| AppError::Parse(format!("Failed to parse Ollama response: {}", e)))?;

    Ok(result.response)
}
