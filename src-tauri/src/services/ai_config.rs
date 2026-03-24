use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;

/// Resolved AI provider configuration.
#[derive(Debug, Clone)]
pub struct ResolvedAiConfig {
    pub api_key: String,
    pub model_name: String,
    pub endpoint_url: String,
}

impl ResolvedAiConfig {
    /// Generate a human-readable model label for persistence (e.g., "claude:claude-sonnet-4-20250514").
    ///
    /// If the user configured a model_name, uses `"{provider}:{model_name}"`.
    /// Otherwise falls back to `"{provider}:default"`.
    pub fn model_label(&self, provider: &str) -> String {
        if self.model_name.is_empty() {
            format!("{}:default", provider)
        } else {
            format!("{}:{}", provider, self.model_name)
        }
    }
}

/// Mirrors AiModelConfig for deserialization from store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAiModel {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub api_key: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub enabled: bool,
}

/// Resolve full config for a given provider from `ai_models` array,
/// with fallback to legacy standalone keys (e.g. `groq_api_key`).
/// Reserved for direct provider lookups (e.g. settings UI, health checks).
#[allow(dead_code)]
pub fn resolve_provider_config(app: &tauri::AppHandle, provider: &str) -> ResolvedAiConfig {
    let empty = ResolvedAiConfig {
        api_key: String::new(),
        model_name: String::new(),
        endpoint_url: String::new(),
    };

    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(_) => return empty,
    };

    // Priority 1: ai_models array — first enabled model matching provider
    if let Some(val) = store.get("ai_models") {
        if let Ok(models) = serde_json::from_value::<Vec<StoredAiModel>>(val.clone()) {
            if let Some(m) = models
                .iter()
                .find(|m| m.provider == provider && m.enabled && !m.api_key.is_empty())
            {
                return ResolvedAiConfig {
                    api_key: m.api_key.clone(),
                    model_name: m.model_name.clone(),
                    endpoint_url: m.endpoint_url.clone(),
                };
            }
        }
    }

    // Priority 2: legacy standalone key
    let legacy_key = match provider {
        "groq" => "groq_api_key",
        "claude" => "claude_api_key",
        _ => return empty,
    };

    let api_key = store
        .get(legacy_key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    ResolvedAiConfig {
        api_key,
        model_name: String::new(),
        endpoint_url: String::new(),
    }
}

/// Convenience: just get the API key for a provider.
/// Reserved for direct provider lookups (e.g. settings UI, health checks).
#[allow(dead_code)]
pub fn resolve_provider_key(app: &tauri::AppHandle, provider: &str) -> String {
    resolve_provider_config(app, provider).api_key
}

/// Resolve config + provider name for batch tasks (news summarization).
///
/// Priority order optimized for 24h unattended operation: deepseek first (cheap, no daily
/// quota limit), then ollama (local, no rate limit), then groq (fast but 14.4k req/day
/// free cap), then others. Returns (config, provider_name).
pub fn resolve_batch_config(app: &tauri::AppHandle) -> (ResolvedAiConfig, String) {
    resolve_config_with_priority(
        app,
        &["deepseek", "ollama", "groq", "openai", "mistral", "claude"],
        "deepseek",
    )
}

/// Resolve config + provider name for deep reasoning.
///
/// Checks `ai_models` for the first enabled model (in priority order: deepseek, claude,
/// groq, ollama, then any other). Returns (config, provider_name).
///
/// DeepSeek is preferred for reasoning: 128K context (fits knowledge base injection),
/// strong reasoning capability, low cost, and Chinese-language strength provides
/// natural bias offset against Western-dominated news sources.
/// Claude serves as fallback / cross-validation when configured.
pub fn resolve_reasoning_config(app: &tauri::AppHandle) -> (ResolvedAiConfig, String) {
    resolve_config_with_priority(
        app,
        &["deepseek", "claude", "groq", "openai", "mistral", "ollama"],
        "deepseek",
    )
}

/// Shared implementation: resolve AI config using a given provider priority order.
///
/// Checks `ai_models` store array for the first enabled model matching the priority list.
/// Falls back to legacy `{fallback_provider}_api_key` store key if no model found.
fn resolve_config_with_priority(
    app: &tauri::AppHandle,
    priority: &[&str],
    fallback_provider: &str,
) -> (ResolvedAiConfig, String) {
    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(_) => {
            return (
                ResolvedAiConfig {
                    api_key: String::new(),
                    model_name: String::new(),
                    endpoint_url: String::new(),
                },
                fallback_provider.to_string(),
            );
        }
    };

    // Check ai_models array for first enabled model with key (priority order)
    if let Some(val) = store.get("ai_models") {
        if let Ok(models) = serde_json::from_value::<Vec<StoredAiModel>>(val.clone()) {
            // First pass: check priority providers in order
            for &provider in priority {
                let needs_key = provider != "ollama";
                if let Some(m) = models
                    .iter()
                    .find(|m| m.provider == provider && m.enabled && (!needs_key || !m.api_key.is_empty()))
                {
                    return (
                        ResolvedAiConfig {
                            api_key: m.api_key.clone(),
                            model_name: m.model_name.clone(),
                            endpoint_url: m.endpoint_url.clone(),
                        },
                        m.provider.clone(),
                    );
                }
            }

            // Second pass: any enabled model not in priority list
            if let Some(m) = models
                .iter()
                .find(|m| m.enabled && !m.api_key.is_empty())
            {
                return (
                    ResolvedAiConfig {
                        api_key: m.api_key.clone(),
                        model_name: m.model_name.clone(),
                        endpoint_url: m.endpoint_url.clone(),
                    },
                    m.provider.clone(),
                );
            }
        }
    }

    // Fallback: try legacy key for fallback provider
    let legacy_key = match fallback_provider {
        "groq" => "groq_api_key",
        "claude" => "claude_api_key",
        _ => "",
    };

    let api_key = if legacy_key.is_empty() {
        String::new()
    } else {
        store
            .get(legacy_key)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default()
    };

    (
        ResolvedAiConfig {
            api_key,
            model_name: String::new(),
            endpoint_url: String::new(),
        },
        fallback_provider.to_string(),
    )
}
