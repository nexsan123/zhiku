use sqlx::SqlitePool;
use std::time::Duration;
use tauri::{Emitter, Manager};

use crate::models::news::{ApiStatus, ApiStatusResponse};
use crate::services::{
    alert_engine, bis_client, coingecko_client, cycle_reasoner, db, deep_analyzer, eia_client,
    fear_greed_client, game_map, global_aggregator, imf_client, indicator_engine, market_context,
    mempool_client, news_cluster, reasoning_scorer, rss_fetcher, scenario_engine, summarizer,
    yahoo_client,
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
    pub bis_interval: Duration,
    pub wto_interval: Duration,
    pub mempool_interval: Duration,
    pub imf_interval: Duration,
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
            bis_interval: Duration::from_secs(6 * 60 * 60),    // 6 hours
            wto_interval: Duration::from_secs(24 * 60 * 60),   // 24 hours
            mempool_interval: Duration::from_secs(5 * 60),     // 5 minutes
            imf_interval: Duration::from_secs(24 * 60 * 60),    // 24 hours (WEO data is annual)
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

    // AI model startup health check (one-shot, 5s delay)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            // Wait 5 seconds for DB and store to be ready
            tokio::time::sleep(Duration::from_secs(5)).await;
            log::info!("PollLoop [AI HealthCheck]: starting startup health checks");

            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("PollLoop [AI HealthCheck]: failed to build HTTP client: {}", e);
                    return;
                }
            };

            // Ollama: GET http://localhost:11434/api/tags (no key needed)
            {
                let start = std::time::Instant::now();
                let (status, error) = match client
                    .get("http://localhost:11434/api/tags")
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        log::info!("PollLoop [AI HealthCheck]: Ollama online");
                        ("online".to_string(), None)
                    }
                    Ok(resp) => {
                        let msg = format!("HTTP {}", resp.status());
                        log::info!("PollLoop [AI HealthCheck]: Ollama responded but not OK: {}", msg);
                        ("offline".to_string(), Some(msg))
                    }
                    Err(e) => {
                        log::info!("PollLoop [AI HealthCheck]: Ollama offline: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "ollama", &status, error.as_deref(), elapsed_ms).await;
            }

            // Groq: GET https://api.groq.com/openai/v1/models with Bearer token
            {
                let config = crate::services::ai_config::resolve_provider_config(&app, "groq");
                let start = std::time::Instant::now();
                let (status, error) = if config.api_key.is_empty() {
                    ("idle".to_string(), Some("API key not configured".to_string()))
                } else {
                    match client
                        .get("https://api.groq.com/openai/v1/models")
                        .bearer_auth(&config.api_key)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            log::info!("PollLoop [AI HealthCheck]: Groq online");
                            ("online".to_string(), None)
                        }
                        Ok(resp) => {
                            let msg = format!("HTTP {}", resp.status());
                            log::info!("PollLoop [AI HealthCheck]: Groq error: {}", msg);
                            ("offline".to_string(), Some(msg))
                        }
                        Err(e) => {
                            log::info!("PollLoop [AI HealthCheck]: Groq offline: {}", e);
                            ("offline".to_string(), Some(e.to_string()))
                        }
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "groq", &status, error.as_deref(), elapsed_ms).await;
            }

            // DeepSeek: GET {endpoint}/v1/models with Bearer token
            {
                let config = crate::services::ai_config::resolve_provider_config(&app, "deepseek");
                let start = std::time::Instant::now();
                let (status, error) = if config.api_key.is_empty() {
                    ("idle".to_string(), Some("API key not configured".to_string()))
                } else {
                    let base = if config.endpoint_url.is_empty() {
                        "https://api.deepseek.com".to_string()
                    } else {
                        config.endpoint_url.trim_end_matches('/').to_string()
                    };
                    let url = format!("{}/v1/models", base);
                    match client
                        .get(&url)
                        .bearer_auth(&config.api_key)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            log::info!("PollLoop [AI HealthCheck]: DeepSeek online");
                            ("online".to_string(), None)
                        }
                        Ok(resp) => {
                            let msg = format!("HTTP {}", resp.status());
                            log::info!("PollLoop [AI HealthCheck]: DeepSeek error: {}", msg);
                            ("offline".to_string(), Some(msg))
                        }
                        Err(e) => {
                            log::info!("PollLoop [AI HealthCheck]: DeepSeek offline: {}", e);
                            ("offline".to_string(), Some(e.to_string()))
                        }
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "deepseek", &status, error.as_deref(), elapsed_ms).await;
            }

            // Claude: GET https://api.anthropic.com/v1/models with x-api-key + anthropic-version
            {
                let config = crate::services::ai_config::resolve_provider_config(&app, "claude");
                let start = std::time::Instant::now();
                let (status, error) = if config.api_key.is_empty() {
                    ("idle".to_string(), Some("API key not configured".to_string()))
                } else {
                    match client
                        .get("https://api.anthropic.com/v1/models")
                        .header("x-api-key", &config.api_key)
                        .header("anthropic-version", "2023-06-01")
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            log::info!("PollLoop [AI HealthCheck]: Claude online");
                            ("online".to_string(), None)
                        }
                        Ok(resp) => {
                            let msg = format!("HTTP {}", resp.status());
                            log::info!("PollLoop [AI HealthCheck]: Claude error: {}", msg);
                            ("offline".to_string(), Some(msg))
                        }
                        Err(e) => {
                            log::info!("PollLoop [AI HealthCheck]: Claude offline: {}", e);
                            ("offline".to_string(), Some(e.to_string()))
                        }
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "claude", &status, error.as_deref(), elapsed_ms).await;
            }

            log::info!("PollLoop [AI HealthCheck]: startup health checks completed");
        });
    }

    // Initial delay: allow tauri-plugin-store and DB pool to fully initialize
    // before any poll task reads API keys from the store.
    // All poll tasks below share this delay by starting after it.
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(8)).await;
            // Emit a ready signal so frontend knows polling is about to start
            let _ = app.emit("poll-loop-ready", true);
            log::info!("PollLoop: store/DB warmup complete, poll tasks will begin fetching");
            // Force an initial status update for all services as "checking"
            for svc in &["rss", "yahoo", "fred", "eia", "fear_greed", "coingecko", "bis", "imf", "wto", "mempool"] {
                update_and_emit(&pool, &app, svc, "checking", None, 0).await;
            }
        });
    }

    // RSS feed polling
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.rss_interval;
        tauri::async_runtime::spawn(async move {
            // Wait for store/DB warmup before first fetch
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                // Read RSSHub base URL from settings store (re-read each cycle for hot-reload)
                let rsshub_raw = read_store_key(&app, "rsshub_base_url");
                let rsshub_base = rss_fetcher::resolve_rsshub_base(&rsshub_raw);
                let start = std::time::Instant::now();
                let (status, error) = match rss_fetcher::fetch_all_rss(&pool, rsshub_base).await {
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
                let (batch_config, batch_provider) = crate::services::ai_config::resolve_batch_config(&app);
                let ai_start = std::time::Instant::now();
                match summarizer::summarize_pending_batch(&pool, &batch_config, &batch_provider).await {
                    Ok(count) => {
                        if count > 0 {
                            log::info!("PollLoop [AI]: {} articles summarized via {}", count, batch_provider);
                            let _ = app.emit("ai-summary-completed", count);
                            // Broadcast to WS clients
                            if let Some(broadcaster) = app.try_state::<crate::services::qt_ws::WsBroadcaster>() {
                                let msg = crate::services::qt_ws::format_ws_event("signal.new", &serde_json::json!({"count": count}));
                                let _ = broadcaster.inner().send(msg);
                            }
                        }
                        let ai_ms = ai_start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, &batch_provider, "online", None, ai_ms).await;
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        log::warn!("PollLoop [AI] summarization error ({}): {}", batch_provider, err_msg);
                        let ai_ms = ai_start.elapsed().as_millis() as i64;
                        update_and_emit(&pool, &app, &batch_provider, "offline", Some(&err_msg), ai_ms)
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
            tokio::time::sleep(Duration::from_secs(10)).await;
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
            tokio::time::sleep(Duration::from_secs(10)).await;
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
                                if count > 0 {
                                    let _ = app.emit("macro-updated", count);
                                }
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
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                let api_key = read_store_key(&app, "eia_api_key");
                let start = std::time::Instant::now();
                let (status, error) = match eia_client::fetch_oil_prices(&pool, &api_key).await {
                    Ok(count) => {
                        if api_key.is_empty() {
                            ("idle".to_string(), Some("API key not configured".to_string()))
                        } else {
                            log::info!("PollLoop [EIA]: {} data points", count);
                            if count > 0 {
                                let _ = app.emit("macro-updated", count);
                            }
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
            tokio::time::sleep(Duration::from_secs(10)).await;
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
            tokio::time::sleep(Duration::from_secs(10)).await;
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

                // Layer 3: AI reasoning (uses user-configured provider/model/endpoint)
                let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);
                let reasoning = match cycle_reasoner::reason_cycle(&indicators, &ai_config, &provider).await {
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
                let model_label = ai_config.model_label(&provider);
                if let Err(e) = cycle_reasoner::persist_reasoning(&pool, &reasoning, &model_label).await {
                    log::warn!("PollLoop [CycleReasoning] persist failed: {}", e);
                }

                // Record scorecard for this reasoning (non-blocking on failure)
                if let Err(e) = reasoning_scorer::record_scorecard(&pool, &reasoning).await {
                    log::warn!("PollLoop [CycleReasoning] scorecard failed: {}", e);
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

                        // Push WS alerts to QuantTerminal
                        push_ws_alerts(&pool, &app).await;
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

    // Two-pass intelligence analysis: clustering + Claude deep analysis (edict-004 Phase C)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let deep_interval = Duration::from_secs(12 * 60 * 60); // 12 hours
        tauri::async_runtime::spawn(async move {
            // Wait 5 minutes for RSS + summarization to produce initial data
            tokio::time::sleep(Duration::from_secs(5 * 60)).await;
            loop {
                log::info!("PollLoop [DeepAnalysis]: starting clustering + deep analysis pass");
                let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);

                // Step 1: Build clusters from recently analyzed news
                match news_cluster::build_clusters(&pool).await {
                    Ok(clusters) => {
                        if clusters.is_empty() {
                            log::info!("PollLoop [DeepAnalysis]: no clusters formed (insufficient data)");
                        } else {
                            log::info!("PollLoop [DeepAnalysis]: {} clusters formed, analyzing...", clusters.len());
                            // Step 2: Deep-analyze each cluster via configured AI provider
                            let model_label = ai_config.model_label(&provider);
                            let mut analyzed = 0usize;
                            for cluster in &clusters {
                                match deep_analyzer::analyze_cluster(&pool, cluster, &ai_config, &provider).await {
                                    Ok(analysis) => {
                                        if let Err(e) = deep_analyzer::persist_analysis(&pool, &analysis, &model_label).await {
                                            log::warn!("PollLoop [DeepAnalysis]: persist failed for cluster {}: {}", cluster.cluster_id, e);
                                        } else {
                                            analyzed += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("PollLoop [DeepAnalysis]: analysis failed for cluster {}: {}", cluster.cluster_id, e);
                                    }
                                }
                            }
                            log::info!("PollLoop [DeepAnalysis]: completed, {}/{} clusters analyzed", analyzed, clusters.len());

                            // Notify frontend
                            if analyzed > 0 {
                                let _ = app.emit("deep-analysis-completed", analyzed);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("PollLoop [DeepAnalysis]: clustering failed: {}", e);
                    }
                }

                tokio::time::sleep(deep_interval).await;
            }
        });
    }

    // Scenario engine auto-update: Claude updates scenario probabilities weekly (edict-004 Phase D)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let scenario_interval = Duration::from_secs(7 * 24 * 60 * 60); // 7 days
        tauri::async_runtime::spawn(async move {
            // Wait 10 minutes for initial data
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            loop {
                log::info!("PollLoop [ScenarioUpdate]: starting weekly scenario probability update");
                let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);

                match scenario_engine::update_scenarios(&pool, &ai_config, &provider).await {
                    Ok(matrix) => {
                        log::info!(
                            "PollLoop [ScenarioUpdate]: updated {} scenarios",
                            matrix.scenarios.len()
                        );
                        let _ = app.emit("scenario-updated", matrix.scenarios.len());
                    }
                    Err(e) => {
                        log::warn!("PollLoop [ScenarioUpdate]: failed: {}", e);
                    }
                }

                tokio::time::sleep(scenario_interval).await;
            }
        });
    }

    // Five-layer reasoning: comprehensive 5-layer synthesis every 12 hours (edict-004 Phase E)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let five_layer_interval = Duration::from_secs(12 * 60 * 60); // 12 hours
        tauri::async_runtime::spawn(async move {
            // Wait 5 minutes for data sources and cycle reasoning to populate first
            tokio::time::sleep(Duration::from_secs(5 * 60)).await;
            loop {
                log::info!("PollLoop [FiveLayerReasoning]: starting five-layer reasoning pass");
                let start = std::time::Instant::now();

                // Build FiveLayerInput from current data
                let indicators = match indicator_engine::calculate_cycle_indicators(&pool).await {
                    Ok(ind) => ind,
                    Err(e) => {
                        log::warn!("PollLoop [FiveLayerReasoning] indicators failed: {}", e);
                        tokio::time::sleep(five_layer_interval).await;
                        continue;
                    }
                };

                let cycle_overview = match global_aggregator::compute_global_overview(&pool).await {
                    Ok(ov) => ov,
                    Err(e) => {
                        log::warn!("PollLoop [FiveLayerReasoning] global overview failed: {}", e);
                        tokio::time::sleep(five_layer_interval).await;
                        continue;
                    }
                };

                // Gather intelligence summaries (latest 5 deep analyses)
                let intelligence_summaries = match deep_analyzer::get_recent_analyses(&pool, 5).await {
                    Ok(analyses) => analyses.iter().map(|a| a.key_observation.clone()).collect(),
                    Err(_) => Vec::new(),
                };

                // Gather active scenario titles
                let active_scenarios = match scenario_engine::get_active_scenarios(&pool).await {
                    Ok(matrix) => matrix.scenarios.iter().map(|s| s.title.clone()).collect(),
                    Err(_) => Vec::new(),
                };

                let input = cycle_reasoner::FiveLayerInput {
                    cycle_overview,
                    indicators,
                    intelligence_summaries,
                    active_scenarios,
                };

                let (ai_config, provider) = crate::services::ai_config::resolve_reasoning_config(&app);
                match cycle_reasoner::reason_five_layer(&pool, &input, &ai_config, &provider).await {
                    Ok(reasoning) => {
                        let model_label = ai_config.model_label(&provider);
                        if let Err(e) = cycle_reasoner::persist_five_layer(&pool, &reasoning, &model_label).await {
                            log::warn!("PollLoop [FiveLayerReasoning] persist failed: {}", e);
                        }

                        // Emit event for frontend
                        app.emit("five-layer-updated", &reasoning)
                            .unwrap_or_else(|e| log::warn!("Failed to emit five-layer-updated: {}", e));

                        log::info!(
                            "PollLoop [FiveLayerReasoning]: completed, confidence={:.2}, took {}ms",
                            reasoning.confidence,
                            start.elapsed().as_millis()
                        );
                    }
                    Err(e) => {
                        log::warn!("PollLoop [FiveLayerReasoning] reasoning failed: {}", e);
                    }
                }

                tokio::time::sleep(five_layer_interval).await;
            }
        });
    }

    // BIS central bank policy rates polling (no API key required)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.bis_interval;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                let start = std::time::Instant::now();
                // Fetch policy rates + credit datasets in one pass
                let mut total = 0usize;
                let mut any_error: Option<String> = None;

                match bis_client::fetch_bis_rates(&pool).await {
                    Ok(n) => total += n,
                    Err(e) => {
                        log::warn!("PollLoop [BIS CBPOL] error: {}", e);
                        any_error = Some(e.to_string());
                    }
                }
                match bis_client::fetch_all_credit_data(&pool).await {
                    Ok(n) => total += n,
                    Err(e) => {
                        log::warn!("PollLoop [BIS Credit] error: {}", e);
                        if any_error.is_none() {
                            any_error = Some(e.to_string());
                        }
                    }
                }

                log::info!("PollLoop [BIS]: {} total observations", total);
                if total > 0 {
                    let _ = app.emit("macro-updated", total);
                }
                let (status, error) = if any_error.is_some() && total == 0 {
                    ("offline".to_string(), any_error)
                } else {
                    ("online".to_string(), None)
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "bis", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // IMF WEO polling (no API key required, annual macro data)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.imf_interval;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match imf_client::fetch_all_data(&pool).await {
                    Ok(count) => {
                        log::info!("PollLoop [IMF]: {} observations", count);
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [IMF] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "imf", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // WTO trade data polling (needs API key)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.wto_interval;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                let api_key = read_store_key(&app, "wto_api_key");
                let start = std::time::Instant::now();
                let (status, error) =
                    match crate::services::wto_client::fetch_wto_data(&pool, &api_key).await {
                        Ok(count) => {
                            if api_key.is_empty() {
                                ("idle".to_string(), Some("API key not configured".to_string()))
                            } else {
                                log::info!("PollLoop [WTO]: {} data points", count);
                                ("online".to_string(), None)
                            }
                        }
                        Err(e) => {
                            log::warn!("PollLoop [WTO] error: {}", e);
                            ("offline".to_string(), Some(e.to_string()))
                        }
                    };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "wto", &status, error.as_deref(), elapsed_ms).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // mempool.space BTC network data polling (no API key required)
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let interval = config.mempool_interval;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                let start = std::time::Instant::now();
                let (status, error) = match mempool_client::fetch_mempool_data(&pool).await {
                    Ok(count) => {
                        log::info!("PollLoop [mempool]: {} indicators updated", count);
                        ("online".to_string(), None)
                    }
                    Err(e) => {
                        log::warn!("PollLoop [mempool] error: {}", e);
                        ("offline".to_string(), Some(e.to_string()))
                    }
                };
                let elapsed_ms = start.elapsed().as_millis() as i64;
                update_and_emit(&pool, &app, "mempool", &status, error.as_deref(), elapsed_ms)
                    .await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    // Scorecard backfill: check for results to score, runs daily
    {
        let pool = pool.clone();
        tauri::async_runtime::spawn(async move {
            // Wait 15 minutes for initial data sources to populate
            tokio::time::sleep(Duration::from_secs(15 * 60)).await;
            let backfill_interval = Duration::from_secs(24 * 60 * 60); // 24 hours
            loop {
                match reasoning_scorer::backfill_actuals(&pool).await {
                    Ok(count) => {
                        if count > 0 {
                            log::info!("PollLoop [Scorecard]: backfilled {} records", count);
                        }
                    }
                    Err(e) => log::warn!("PollLoop [Scorecard]: backfill failed: {}", e),
                }
                tokio::time::sleep(backfill_interval).await;
            }
        });
    }

    // Daily brief: generate comprehensive intelligence brief every 6 hours
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let brief_interval = Duration::from_secs(6 * 60 * 60);
        tauri::async_runtime::spawn(async move {
            // Wait 3 minutes for other data sources to populate first
            tokio::time::sleep(Duration::from_secs(3 * 60)).await;
            loop {
                log::info!("PollLoop [DailyBrief]: generating intelligence brief");
                match crate::services::daily_brief::generate_daily_brief(&pool).await {
                    Ok(brief) => {
                        log::info!(
                            "PollLoop [DailyBrief]: generated, headline={}",
                            brief.headline
                        );
                        app.emit("daily-brief-updated", &brief)
                            .unwrap_or_else(|e| log::warn!("Failed to emit daily-brief: {}", e));
                    }
                    Err(e) => log::warn!("PollLoop [DailyBrief]: failed: {}", e),
                }
                tokio::time::sleep(brief_interval).await;
            }
        });
    }

    // Alert engine: check threshold rules every 30 minutes
    {
        let pool = pool.clone();
        let app = app_handle.clone();
        let alert_interval = Duration::from_secs(30 * 60);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2 * 60)).await; // 2min initial delay
            loop {
                match alert_engine::check_alerts(&pool).await {
                    Ok(alerts) => {
                        if !alerts.is_empty() {
                            log::info!(
                                "PollLoop [AlertEngine]: {} new alerts triggered",
                                alerts.len()
                            );
                            for alert in &alerts {
                                log::info!(
                                    "  ALERT [{}] {}: {}",
                                    alert.severity,
                                    alert.title,
                                    alert.detail
                                );
                            }
                            // Emit to frontend
                            app.emit("alerts-triggered", &alerts).unwrap_or_else(|e| {
                                log::warn!("Failed to emit alerts: {}", e)
                            });

                            // Send desktop notification for critical alerts
                            for alert in &alerts {
                                if alert.severity == "critical" {
                                    app.emit(
                                        "show-notification",
                                        serde_json::json!({
                                            "title": alert.title,
                                            "body": &alert.detail,
                                        }),
                                    )
                                    .unwrap_or_else(|e| {
                                        log::warn!("Notification emit failed: {}", e)
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => log::warn!("PollLoop [AlertEngine]: {}", e),
                }
                tokio::time::sleep(alert_interval).await;
            }
        });
    }

    // Trend tracker: snapshot computed indicators every 6 hours
    {
        let pool = pool.clone();
        let snapshot_interval = Duration::from_secs(6 * 60 * 60);
        tauri::async_runtime::spawn(async move {
            // Wait 4 minutes for indicator data sources to populate first
            tokio::time::sleep(Duration::from_secs(4 * 60)).await;
            loop {
                match crate::services::trend_tracker::snapshot_indicators(&pool).await {
                    Ok(count) => log::info!("PollLoop [TrendTracker]: snapshot {} indicators", count),
                    Err(e) => log::warn!("PollLoop [TrendTracker]: {}", e),
                }
                tokio::time::sleep(snapshot_interval).await;
            }
        });
    }

    log::info!("SmartPollLoop: all 22 tasks spawned (1 ai-health-check + 12 data + 1 cleanup + cycle-reasoning + five-layer-reasoning + scorecard-backfill + market-context + deep-analysis + scenario-engine + daily-brief + alert-engine + trend-tracker)");
}

/// Push WS alerts to QuantTerminal after MarketContext write.
///
/// Three event types (edict-004 Phase F):
/// 1. `turning.signal` — high-confidence turning signals from cycle reasoning
/// 2. `calendar.alert` — decision calendar events within 48 hours
/// 3. `scenario.update` — current scenario matrix for QT to track changes
async fn push_ws_alerts(pool: &SqlitePool, app: &tauri::AppHandle) {
    let broadcaster = match app.try_state::<crate::services::qt_ws::WsBroadcaster>() {
        Some(b) => b,
        None => return,
    };

    // 1. High-confidence turning signals
    if let Ok(Some(reasoning)) = cycle_reasoner::get_latest_reasoning(pool).await {
        // Only push if confidence >= 0.7 and there are turning signals
        if reasoning.confidence >= 0.7 && !reasoning.turning_signals.is_empty() {
            let msg = crate::services::qt_ws::format_ws_event(
                "turning.signal",
                &serde_json::json!({
                    "signals": reasoning.turning_signals,
                    "confidence": reasoning.confidence,
                }),
            );
            let _ = broadcaster.inner().send(msg);
        }
    }

    // 2. Decision calendar alerts — events within 48 hours
    if let Ok(events) = game_map::get_calendar_events(2) {
        if !events.is_empty() {
            let msg = crate::services::qt_ws::format_ws_event(
                "calendar.alert",
                &serde_json::json!({
                    "events": events,
                    "count": events.len(),
                    "horizon": "48h",
                }),
            );
            let _ = broadcaster.inner().send(msg);
        }
    }

    // 3. Scenario matrix update — push current state for QT to track changes
    if let Ok(scenarios) = scenario_engine::get_active_scenarios(pool).await {
        if !scenarios.scenarios.is_empty() {
            let msg = crate::services::qt_ws::format_ws_event("scenario.update", &scenarios);
            let _ = broadcaster.inner().send(msg);
        }
    }
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

    // Emit event to frontend (enriched with freshness)
    let raw = ApiStatus {
        service: service.to_string(),
        status: status.to_string(),
        last_check: Some(now),
        last_error: error.map(|s| s.to_string()),
        response_ms: Some(response_ms),
    };
    let payload = ApiStatusResponse::from(raw);

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
