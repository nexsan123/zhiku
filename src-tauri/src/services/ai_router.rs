//! Unified AI routing layer.
//!
//! All AI calls (reasoning, summarization, deep analysis) go through
//! `ai_router::reason()`. The router dispatches to the correct backend:
//!
//! - "claude" → Anthropic Messages API (claude_client)
//! - everything else → OpenAI-compatible API (openai_compat)

use crate::errors::AppError;
use crate::services::ai_config::ResolvedAiConfig;
use crate::services::{claude_client, openai_compat};

/// Send a reasoning/analysis request to the user-configured AI provider.
///
/// This is the single entry point for all AI reasoning chains (cycle reasoning,
/// five-layer reasoning, deep analysis, scenario engine, news summarization).
///
/// # Arguments
/// * `prompt` - User message content
/// * `system` - Optional system prompt (role definition + output format)
/// * `config` - Resolved config from settings (api_key, model_name, endpoint_url)
/// * `provider` - Provider name: "claude", "groq", "ollama", "deepseek", "mistral", etc.
pub async fn reason(
    prompt: &str,
    system: Option<&str>,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<String, AppError> {
    match provider {
        "claude" => claude_client::analyze(prompt, system, config).await,
        _ => openai_compat::chat_completion(prompt, system, config, provider).await,
    }
}
