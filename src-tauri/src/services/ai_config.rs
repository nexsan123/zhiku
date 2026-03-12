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
pub fn resolve_provider_key(app: &tauri::AppHandle, provider: &str) -> String {
    resolve_provider_config(app, provider).api_key
}
