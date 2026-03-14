use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri_plugin_store::StoreExt;

use crate::services::rss_fetcher::RSS_SOURCES;

/// Connection test result returned to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub response_ms: u64,
}

/// RSS source info returned to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RssSourceInfo {
    pub url: String,
    pub name: String,
    pub tier: u8,
    pub language: String,
    pub enabled: bool,
}

/// AI model configuration stored in settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelConfig {
    pub id: String,
    pub provider: String,      // ollama, groq, claude, openai, gemini, deepseek, openai_compatible
    pub display_name: String,
    pub api_key: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub enabled: bool,
}

/// Known settings keys (for enumeration since store may not support keys()).
const KNOWN_KEYS: &[&str] = &[
    "fred_api_key",
    "eia_api_key",
    "wto_api_key",
    "groq_api_key",
    "claude_api_key",
    "ollama_base_url",
    "disabled_rss_urls",
];

/// Mask an API key for display: show first 3 + last 4 chars.
fn mask_value(value: &str) -> String {
    if value.len() <= 8 {
        return "***".to_string();
    }
    format!("{}***{}", &value[..3], &value[value.len() - 4..])
}

fn is_api_key(key: &str) -> bool {
    key.ends_with("_api_key")
}

fn read_store_key(app: &tauri::AppHandle, key: &str) -> String {
    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    store
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

// ─────────────────────── Commands ───────────────────────

/// Read all known settings. API key values are masked.
#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<HashMap<String, String>, String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let mut result = HashMap::new();

    for &key in KNOWN_KEYS {
        if let Some(value) = store.get(key) {
            if let Some(s) = value.as_str() {
                if !s.is_empty() {
                    let display = if is_api_key(key) {
                        mask_value(s)
                    } else {
                        s.to_string()
                    };
                    result.insert(key.to_string(), display);
                }
            }
        }
    }

    Ok(result)
}

/// Write a single setting to the store.
#[tauri::command]
pub async fn set_setting(
    app: tauri::AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set(&key, serde_json::Value::String(value));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a single setting from the store.
#[tauri::command]
pub async fn delete_setting(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let _ = store.delete(&key);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Test connectivity to an external service.
#[tauri::command]
pub async fn test_connection(
    app: tauri::AppHandle,
    service: String,
    api_key: Option<String>,
) -> Result<TestResult, String> {
    let start = std::time::Instant::now();

    let get_key = |store_key: &str| -> String {
        api_key
            .clone()
            .unwrap_or_else(|| read_store_key(&app, store_key))
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let result: Result<String, String> = match service.as_str() {
        "ollama" => {
            // Inline health check: GET http://localhost:11434/api/tags
            match client.get("http://localhost:11434/api/tags").send().await {
                Ok(r) if r.status().is_success() => Ok("Ollama is running".into()),
                Ok(_) => Err("Ollama is not responding".into()),
                Err(e) => Err(format!("Ollama error: {}", e)),
            }
        }
        "groq" => {
            let key = get_key("groq_api_key");
            if key.is_empty() {
                Err("Groq API key not configured".into())
            } else {
                match client
                    .post("https://api.groq.com/openai/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", key))
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "model": "llama-3.1-8b-instant",
                        "messages": [{"role": "user", "content": "ping"}],
                        "max_tokens": 5
                    }))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => Ok("Groq API connected".into()),
                    Ok(r) => Err(format!("Groq API error: {}", r.status())),
                    Err(e) => Err(format!("Groq network error: {}", e)),
                }
            }
        }
        "claude" => {
            let key = get_key("claude_api_key");
            if key.is_empty() {
                Err("Claude API key not configured".into())
            } else {
                match client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &key)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "model": "claude-sonnet-4-20250514",
                        "max_tokens": 5,
                        "messages": [{"role": "user", "content": "ping"}]
                    }))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => Ok("Claude API connected".into()),
                    Ok(r) => Err(format!("Claude API error: {}", r.status())),
                    Err(e) => Err(format!("Claude network error: {}", e)),
                }
            }
        }
        "fred" => {
            let key = get_key("fred_api_key");
            if key.is_empty() {
                Err("FRED API key not configured".into())
            } else {
                let url = format!(
                    "https://api.stlouisfed.org/fred/series?series_id=FEDFUNDS&api_key={}&file_type=json",
                    key
                );
                match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => Ok("FRED API connected".into()),
                    Ok(r) => Err(format!("FRED API error: {}", r.status())),
                    Err(e) => Err(format!("FRED network error: {}", e)),
                }
            }
        }
        "eia" => {
            let key = get_key("eia_api_key");
            if key.is_empty() {
                Err("EIA API key not configured".into())
            } else {
                let url = format!("https://api.eia.gov/v2/?api_key={}", key);
                match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => Ok("EIA API connected".into()),
                    Ok(r) => Err(format!("EIA API error: {}", r.status())),
                    Err(e) => Err(format!("EIA network error: {}", e)),
                }
            }
        }
        "wto" => {
            let key = get_key("wto_api_key");
            if key.is_empty() {
                Err("WTO API key not configured".into())
            } else {
                let url = format!(
                    "https://api.wto.org/timeseries/v1/data?i=ITS_MTV_AX.A.A&r=000&ps=2023&max=1&subscription-key={}",
                    key
                );
                match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => Ok("WTO API connected".into()),
                    Ok(r) => Err(format!("WTO API error: {}", r.status())),
                    Err(e) => Err(format!("WTO network error: {}", e)),
                }
            }
        }
        _ => Err(format!("Unknown service: {}", service)),
    };

    let elapsed = start.elapsed().as_millis() as u64;

    Ok(match result {
        Ok(msg) => TestResult {
            success: true,
            message: msg,
            response_ms: elapsed,
        },
        Err(msg) => TestResult {
            success: false,
            message: msg,
            response_ms: elapsed,
        },
    })
}

// ─────────────────────── AI Model Management ───────────────────────

fn read_ai_models(app: &tauri::AppHandle) -> Vec<AiModelConfig> {
    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    store
        .get("ai_models")
        .and_then(|v| serde_json::from_value::<Vec<AiModelConfig>>(v.clone()).ok())
        .unwrap_or_default()
}

fn write_ai_models(app: &tauri::AppHandle, models: &[AiModelConfig]) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let val = serde_json::to_value(models).map_err(|e| e.to_string())?;
    store.set("ai_models", val);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// List all configured AI models. API keys are masked.
#[tauri::command]
pub async fn list_ai_models(app: tauri::AppHandle) -> Result<Vec<AiModelConfig>, String> {
    let models = read_ai_models(&app);
    let masked: Vec<AiModelConfig> = models
        .into_iter()
        .map(|mut m| {
            if !m.api_key.is_empty() {
                m.api_key = mask_value(&m.api_key);
            }
            m
        })
        .collect();
    Ok(masked)
}

/// Save (add or update) an AI model configuration.
#[tauri::command]
pub async fn save_ai_model(app: tauri::AppHandle, model: AiModelConfig) -> Result<(), String> {
    let mut models = read_ai_models(&app);

    // If updating existing, preserve the old API key if new one looks masked
    if let Some(existing) = models.iter().find(|m| m.id == model.id) {
        if model.api_key.contains("***") || model.api_key.is_empty() {
            // Keep old key
            let old_key = existing.api_key.clone();
            models.retain(|m| m.id != model.id);
            let mut updated = model;
            updated.api_key = old_key;
            models.push(updated);
            write_ai_models(&app, &models)?;
            return Ok(());
        }
    }

    models.retain(|m| m.id != model.id);
    models.push(model);
    write_ai_models(&app, &models)?;
    Ok(())
}

/// Remove an AI model by id.
#[tauri::command]
pub async fn remove_ai_model(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut models = read_ai_models(&app);
    models.retain(|m| m.id != id);
    write_ai_models(&app, &models)?;
    Ok(())
}

/// Test an AI model connection dynamically using its config.
#[tauri::command]
pub async fn test_ai_model(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<TestResult, String> {
    let models = read_ai_models(&app);
    let model = models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model not found: {}", model_id))?
        .clone();

    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let result: Result<String, String> = match model.provider.as_str() {
        "ollama" => {
            let base = if model.endpoint_url.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                model.endpoint_url.trim_end_matches('/').to_string()
            };
            match client.get(format!("{}/api/tags", base)).send().await {
                Ok(r) if r.status().is_success() => Ok("Ollama is running".into()),
                Ok(r) => Err(format!("Ollama error: {}", r.status())),
                Err(e) => Err(format!("Ollama not reachable: {}", e)),
            }
        }
        "groq" | "openai" | "deepseek" | "openai_compatible" => {
            // All OpenAI-compatible APIs
            if model.api_key.is_empty() {
                Err(format!("{} API key not configured", model.display_name))
            } else {
                let endpoint = if model.endpoint_url.is_empty() {
                    match model.provider.as_str() {
                        "groq" => "https://api.groq.com/openai/v1/chat/completions".to_string(),
                        "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
                        "deepseek" => "https://api.deepseek.com/v1/chat/completions".to_string(),
                        _ => return Ok(TestResult {
                            success: false,
                            message: "Endpoint URL required for custom provider".into(),
                            response_ms: 0,
                        }),
                    }
                } else {
                    let base = model.endpoint_url.trim_end_matches('/');
                    if base.ends_with("/chat/completions") {
                        base.to_string()
                    } else {
                        format!("{}/chat/completions", base)
                    }
                };

                let model_name = if model.model_name.is_empty() {
                    match model.provider.as_str() {
                        "groq" => "llama-3.1-8b-instant",
                        "openai" => "gpt-4o-mini",
                        "deepseek" => "deepseek-chat",
                        _ => "default",
                    }
                    .to_string()
                } else {
                    model.model_name.clone()
                };

                match client
                    .post(&endpoint)
                    .header("Authorization", format!("Bearer {}", model.api_key))
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "model": model_name,
                        "messages": [{"role": "user", "content": "ping"}],
                        "max_tokens": 5
                    }))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => {
                        Ok(format!("{} connected", model.display_name))
                    }
                    Ok(r) => Err(format!("{} error: {}", model.display_name, r.status())),
                    Err(e) => Err(format!("{} network error: {}", model.display_name, e)),
                }
            }
        }
        "claude" => {
            if model.api_key.is_empty() {
                Err("Claude API key not configured".into())
            } else {
                let endpoint = if model.endpoint_url.is_empty() {
                    "https://api.anthropic.com/v1/messages".to_string()
                } else {
                    model.endpoint_url.clone()
                };

                match client
                    .post(&endpoint)
                    .header("x-api-key", &model.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "model": if model.model_name.is_empty() { "claude-sonnet-4-20250514" } else { &model.model_name },
                        "max_tokens": 5,
                        "messages": [{"role": "user", "content": "ping"}]
                    }))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => Ok("Claude API connected".into()),
                    Ok(r) => Err(format!("Claude API error: {}", r.status())),
                    Err(e) => Err(format!("Claude network error: {}", e)),
                }
            }
        }
        "gemini" => {
            if model.api_key.is_empty() {
                Err("Gemini API key not configured".into())
            } else {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    if model.model_name.is_empty() { "gemini-pro" } else { &model.model_name },
                    model.api_key
                );
                match client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "contents": [{"parts": [{"text": "ping"}]}]
                    }))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => Ok("Gemini API connected".into()),
                    Ok(r) => Err(format!("Gemini API error: {}", r.status())),
                    Err(e) => Err(format!("Gemini network error: {}", e)),
                }
            }
        }
        _ => Err(format!("Unknown provider: {}", model.provider)),
    };

    let elapsed = start.elapsed().as_millis() as u64;
    Ok(match result {
        Ok(msg) => TestResult { success: true, message: msg, response_ms: elapsed },
        Err(msg) => TestResult { success: false, message: msg, response_ms: elapsed },
    })
}

/// Return all RSS sources with enabled/disabled status.
#[tauri::command]
pub async fn get_rss_sources(app: tauri::AppHandle) -> Result<Vec<RssSourceInfo>, String> {
    let disabled: Vec<String> = {
        let store = app.store("settings.json").map_err(|e| e.to_string())?;
        store
            .get("disabled_rss_urls")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .unwrap_or_default()
    };

    let sources: Vec<RssSourceInfo> = RSS_SOURCES
        .iter()
        .map(|s| RssSourceInfo {
            url: s.url.to_string(),
            name: s.name.to_string(),
            tier: s.tier as u8,
            language: s.language.to_string(),
            enabled: !disabled.contains(&s.url.to_string()),
        })
        .collect();

    Ok(sources)
}
