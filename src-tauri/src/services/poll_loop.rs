use sqlx::SqlitePool;
use std::time::Duration;
use tauri::{Emitter, Manager};

use crate::models::news::ApiStatus;
use crate::services::{
    coingecko_client, cycle_reasoner, db, eia_client, fear_greed_client, indicator_engine,
    market_context, rss_fetcher, summarizer, yahoo_client,
};

/// Configuration for poll intervals per data source.
pub struct PollConfig {
    pub rss_interval: Duration,
    pub yahoo_interval: Duration,
    pub fred_interval: Duration,
    pub eia_interval: Duration,
    pub fear_greed_interval: Duration,
    pub coingecko_interval: Duration,
    pub mc_interval: Duration,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            rss_interval: Duration::from_secs(5 * 60),       // 5 minutes
            yahoo_interval: Duration::from_secs(60),          // 1 minute
            fred_interval: Duration::from_secs(60 * 60),      // 1 hour
            eia_interval: Duration::from_secs(6 * 60 * 60),   // 6 hours
            fear_greed_interval: Duration::from_secs(30 * 60), // 30 minutes
            coingecko_interval: Duration::from_secs(2 * 60),  // 2 minutes
            mc_interval: Duration::from_secs(60),               // 1 minute
        }
    }
}

/// Start the SmartPollLoop: spawn independent background tasks for each data source.
///
/// Uses `tauri::async_runtime::spawn` per RT-001 (not tokio::spawn).
/// Each data source runs at its own interval. Errors are logged, never panic.
/// After each fetch, updates `api_status` table and emits `api-status-changed` event.
pub fn start_poll_loop(app_handle: tauri::AppHandle, pool: SqlitePool, mc_pool: SqlitePool, config: PollConfig) {
    log::info!("SmartPollLoop starting...");

    // RSS feed polling
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.rss_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match rss_fetcher::fetch_all_rss(&pool).await {
                    Ok(count) => {
                        log::info!("PollLoop [RSS]: {} new articles", count);
                        if count > 0 {
                            let _ = app.emit("news-updated", count);
                        }
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [RSS] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "rss", &status, error.as_deref(), elapsed_ms).await;

                // Phase 3: AI summarization of pending news after RSS fetch
                let groq_key = read_store_key(&app, "groq_api_key");
                let ai_start = std::time::Instant::now();
                match summarizer::summarize_pending_batch(&pool, &groq_key).await {
                    Ok(count) => {
                        if count > 0 {
                            log::info!("PollLoop [AI]: {} articles summarized", count);
                            let _ = app.emit("ai-summary-completed", count);
                            // Broadcast to WS clients
                            if let Some(broadcaster) = app.try_state::<crate::services::qt_ws::WsBroadcaster>() {
                                let msg = crate::services::qt_ws::format_ws_event("signal.new", &serde_json::json!({"count": count}));
                                let _ = broadcaster.inner().send(msg);
                            }
                        }
                        let ai_ms = ai_start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, "ollama", "online", None, ai_ms).await;
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        log::warn!("PollLoop [AI] summarization error: {}", err_msg);
                        let ai_ms = ai_start.elapsed().as_millis() as i64;
                        // Update ollama/groq status based on error
                        update_and_emit(&pool, &app, "ollama", "offline", Some(&err_msg), ai_ms)
                            .await;
                        update_and_emit(&pool, &app, "groq", "offline", Some(&err_msg), ai_ms)
                            .await;
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    // Yahoo Finance polling
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.yahoo_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match yahoo_client::fetch_all_quotes(&pool).await {
                    Ok(count) => {
                        log::info!("PollLoop [Yahoo]: {} snapshots", count);
                        if count > 0 {
                            let _ = app.emit("market-updated", count);
                        }
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [Yahoo] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "yahoo", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // FRED polling (needs API key from store)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.fred_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let api_key = read_store_key(&app, "fred_api_key");
                let start = std::time::Instant::now();
                let (status, error) =
                    match crate::services::fred_client::fetch_all_series(&pool, &api_key).await {
                        Ok(count) => {
                            if api_key.is_empty() {
                                ("idle".to_string(), Some("API key not configured".to_string()))
                            } else {
                                log::info!("PollLoop [FRED]: {} observations", count);
                                ("online".to_string(), None)
                            }
                        }
                        Err(e) => {
                            log::warn!("PollLoop [FRED] error: {}", e);
                            ("offline".to_string(), Some(e.to_string()))
                        }
                    };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "fred", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // EIA polling (needs API key)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.eia_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let api_key = read_store_key(&app, "eia_api_key");
                let start = std::time::Instant::now();
                let (status, error) = match eia_client::fetch_oil_prices(&pool, &api_key).await {
                    Ok(count) => {
                        if api_key.is_empty() {
                            ("idle".to_string(), Some("API key not configured".to_string()))
                        } else {
                            log::info!("PollLoop [EIA]: {} data points", count);
                            ("online".to_string(), None)
                        }
                    }
                    Err(e) => {
                        log::warn!("PollLoop [EIA] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "eia", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // Fear & Greed polling
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.fear_greed_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match fear_greed_client::fetch_fear_greed(&pool).await {
                    Ok(_count) => {
                        log::info!("PollLoop [Fear&Greed]: updated");
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [Fear&Greed] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "fear_greed", &status, error.as_deref(), elapsed_ms)
                    .await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // CoinGecko polling
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.coingecko_interval;
        tauri::async_runtime::spawn(async move {
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match coingecko_client::fetch_crypto_prices(&pool).await {
                    Ok(count) => {
                        log::info!("PollLoop [CoinGecko]: {} snapshots", count);
                        if count > 0 {
                            let _ = app.emit("market-updated", count);
                        }
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [CoinGecko] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "coingecko", &status, error.as_deref(), elapsed_ms)
                    .await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // market_snap cleanup task (BUG-010): delete rows older than 72 hours, runs every hour
    {
        let pool = pool.clone();
        tauri::async_runtime::spawn(async move {
            let cleanup_interval = Duration::from_secs(60 * 60); // 1 hour
            loop {
                tokio::time::sleep(cleanup_interval).await;
                match db::cleanup_old_market_snaps(&pool, 72).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            log::info!("PollLoop [Cleanup]: deleted {} old market_snap rows", deleted);
                        }
                    }
                    Err(e) => {
                        log::warn!("PollLoop [Cleanup] error: {}", e);
                    }
                }
            }
        });
    }

    // Cycle reasoning task: compute indicators + Claude reasoning every 6 hours
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let cycle_interval = Duration::from_secs(6 * 60 * 60); // 6 hours
        tauri::async_runtime::spawn(async move {
            // Initial delay: wait 2 minutes for data sources to populate first
            tokio::time::sleep(Duration::from_secs(120)).await;
            loop {
                log::info!("PollLoop [CycleReasoning]: starting cycle reasoning pass");
                let start = std::time::Instant::now();

                // Layer 2: compute indicators
                let indicators = match indicator_engine::calculate_cycle_indicators(&pool).await {
                    Ok(ind) => ind,
                    Err(e) => {
                        log::warn!("PollLoop [CycleReasoning] indicator calculation failed: {}", e);
                        let elapsed_ms = start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, "claude", "offline", Some(&e.to_string()), elapsed_ms).await;
                        tokio::time::sleep(cycle_interval).await;
                        continue;
                    }
                };

                // Layer 3: Claude reasoning
                let claude_key = read_store_key(&app, "claude_api_key");
                let reasoning = match cycle_reasoner::reason_cycle(&indicators, &claude_key).await {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!("PollLoop [CycleReasoning] reasoning failed: {}", e);
                        let elapsed_ms = start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, "claude", "offline", Some(&e.to_string()), elapsed_ms).await;
                        tokio::time::sleep(cycle_interval).await;
                        continue;
                    }
                };

                // Layer 4: persist
                if let Err(e) = cycle_reasoner::persist_reasoning(&pool, &reasoning).await {
                    log::warn!("PollLoop [CycleReasoning] persist failed: {}", e);
                }

                let elapsed_ms = start.elapsed().as_millis() as i64;
                let status = if reasoning.confidence > 0.0 { "online" } else { "idle" };
                let error_msg = if reasoning.confidence == 0.0 {
                    Some("No API key or default reasoning".to_string())
                } else {
                    None
                };
                update_and_emit(&pool, &app, "claude", status, error_msg.as_deref(), elapsed_ms).await;

                // Emit cycle-reasoning-updated event
                app.emit("cycle-reasoning-updated", &reasoning)
                    .unwrap_or_else(|e| {
                        log::warn!("Failed to emit cycle-reasoning-updated: {}", e);
                    });

                // Broadcast to WS clients
                if let Some(broadcaster) = app.try_state::<crate::services::qt_ws::WsBroadcaster>() {
                    let msg = crate::services::qt_ws::format_ws_event("cycle.update", &reasoning);
                    let _ = broadcaster.inner().send(msg);
                }

                log::info!(
                    "PollLoop [CycleReasoning]: completed, confidence={:.2}, took {}ms",
                    reasoning.confidence,
                    elapsed_ms
                );

                tokio::time::sleep(cycle_interval).await;
            }
        });
    }

    // MarketContext writer -- push to shared SQLite for QuantTerminal
    {
        let pool = pool.clone();
        let mc_pool = mc_pool;
        let app = app_handle.clone();
        let interval = config.mc_interval;
        tauri::async_runtime::spawn(async move {
            // Wait 30 seconds for data sources to populate first
            tokio::time::sleep(Duration::from_secs(30)).await;
            loop {
                let start = std::time::Instant::now();
                match market_context::write_market_context(&mc_pool, &pool).await {
                    Ok(()) => {
                        let elapsed_ms = start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, "qt_rest", "online", None, elapsed_ms).await;
                    }
                    Err(e) => {
                        log::warn!("PollLoop [MarketContext] error: {}", e);
                        let elapsed_ms = start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, "qt_rest", "offline", Some(&e.to_string()), elapsed_ms).await;
                    }
                }
                tokio::time::sleep(interval).await;
            }
        });
    }

    log::info!("SmartPollLoop: all 9 tasks spawned (6 data sources + 1 cleanup + AI summarization + cycle reasoning + market context)");
}

/// Update api_status table and emit event to frontend.
async fn update_and_emit(
    pool: &SqlitePool,
    app: &tauri::AppHandle,
    service: &str,
    status: &str,
    error: Option<&str>,
    response_ms: i64,
) {
    let now = chrono::Utc::now().to_rfc3339();

    // Update api_status table
    let result = sqlx::query(
        r#"INSERT INTO api_status (service, status, last_check, last_error, response_ms)
           VALUES (?1, ?2, ?3, ?4, ?5)
           ON CONFLICT(service) DO UPDATE SET
             status = excluded.status,
             last_check = excluded.last_check,
             last_error = excluded.last_error,
             response_ms = excluded.response_ms"#,
    )
    .bind(service)
    .bind(status)
    .bind(&now)
    .bind(error)
    .bind(response_ms)
    .execute(pool)
    .await;

    if let Err(e) = result {
        log::warn!("Failed to update api_status for {}: {}", service, e);
    }

    // Emit event to frontend
    let payload = ApiStatus {
        service: service.to_string(),
        status: status.to_string(),
        last_check: Some(now),
        last_error: error.map(|s| s.to_string()),
        response_ms: Some(response_ms),
    };

    app.emit("api-status-changed", &payload)
        .unwrap_or_else(|e| {
            log::warn!("Failed to emit api-status-changed for {}: {}", service, e);
        });
}

/// Read a key from tauri-plugin-store settings.json.
/// Returns empty string if key not found or store unavailable.
fn read_store_key(app: &tauri::AppHandle, key: &str) -> String {
    use tauri_plugin_store::StoreExt;

    let store = match app.store("settings.json") {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to open settings store for {}: {}", key, e);
            return String::new();
        }
    };

    store
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}
