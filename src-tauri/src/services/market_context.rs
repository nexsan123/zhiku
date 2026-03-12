use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::credit::{CreditCyclePhase, TideState};
use crate::services::{game_map, global_aggregator};

/// Initialize the market_context.db shared SQLite file.
/// Creates the file and schema if it doesn't exist.
/// QuantTerminal polls this file by mtime to read market context.
pub async fn init_market_context_db(path: &std::path::Path) -> Result<SqlitePool, AppError> {
    // Ensure parent dir exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db_url = format!("sqlite:{}?mode=rwc", path.display());
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| AppError::Database(format!("market_context.db connect failed: {}", e)))?;

    // Create table with regime columns for QuantTerminal's RegimeDispatcher
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS market_context (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            regime TEXT NOT NULL DEFAULT 'neutral',
            event_risk TEXT NOT NULL DEFAULT 'none',
            vix_level REAL,
            sector_bias TEXT,
            news_sentiment REAL,
            upcoming_events TEXT DEFAULT '[]',
            summary TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT 'zhiku',
            schema_version INTEGER NOT NULL DEFAULT 2,
            -- Regime columns for QuantTerminal's RegimeDispatcher (7 fields)
            market_regime TEXT NOT NULL DEFAULT 'trending_up',
            regime_confidence REAL NOT NULL DEFAULT 0.5,
            regime_reasoning TEXT NOT NULL DEFAULT '',
            regime_trend TEXT NOT NULL DEFAULT 'stable',
            regime_hist_median_days REAL,
            regime_hist_min_days REAL,
            regime_hist_max_days REAL
        )"#,
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(format!("market_context schema creation failed: {}", e)))?;

    // Migrate existing tables: add regime columns if they don't exist
    for col_def in &[
        "market_regime TEXT NOT NULL DEFAULT 'trending_up'",
        "regime_confidence REAL NOT NULL DEFAULT 0.5",
        "regime_reasoning TEXT NOT NULL DEFAULT ''",
        "regime_trend TEXT NOT NULL DEFAULT 'stable'",
        "regime_hist_median_days REAL",
        "regime_hist_min_days REAL",
        "regime_hist_max_days REAL",
    ] {
        let col_name = col_def.split_whitespace().next().unwrap_or("");
        let sql = format!("ALTER TABLE market_context ADD COLUMN {}", col_def);
        // Ignore "duplicate column" errors for idempotent migration
        if let Err(e) = sqlx::query(&sql).execute(&pool).await {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column") {
                log::warn!("market_context migration for '{}': {}", col_name, err_str);
            }
        }
    }

    // Enable WAL mode for concurrent read/write (智库 writes, QT reads simultaneously)
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .ok();

    log::info!("market_context.db initialized at {}", path.display());
    Ok(pool)
}

/// Map 智库's global credit cycle phase to QuantTerminal's 4-state MarketRegime.
///
/// Mapping logic:
/// - easing / leveraging → trending_up (expansion phase)
/// - overheating → volatile_up (late cycle, elevated risk)
/// - tightening → trending_down (contraction begins)
/// - deleveraging / clearing → crisis (stress/deleveraging)
/// - unknown → use VIX fallback
fn map_cycle_to_regime(global_phase: &CreditCyclePhase, vix: Option<f64>) -> &'static str {
    // VIX override: extreme levels always dominate
    if let Some(v) = vix {
        if v > 35.0 {
            return "crisis";
        }
    }

    match global_phase {
        CreditCyclePhase::Easing | CreditCyclePhase::Leveraging => "trending_up",
        CreditCyclePhase::Overheating => "volatile_up",
        CreditCyclePhase::Tightening => "trending_down",
        CreditCyclePhase::Deleveraging | CreditCyclePhase::Clearing => "crisis",
        CreditCyclePhase::Unknown => {
            // Fallback for unknown: use VIX-based heuristic
            match vix {
                Some(v) if v > 25.0 => "trending_down",
                Some(v) if v < 18.0 => "trending_up",
                _ => "trending_up", // safe default
            }
        }
    }
}

/// Derive regime trend from dollar tide + credit impulse direction.
fn derive_regime_trend(tide_state: &TideState, global_phase: &CreditCyclePhase) -> &'static str {
    use CreditCyclePhase::*;
    use TideState::*;
    match (global_phase, tide_state) {
        // Expansion + supportive tide = stabilizing
        (Easing | Leveraging, Ebbing) => "stabilizing",
        // Expansion + rising tide = stable
        (Easing | Leveraging, Rising) => "stable",
        // Contraction + rising tide = double pressure
        (Tightening | Deleveraging, Rising) => "worsening",
        // Contraction + ebbing tide = relief
        (Tightening | Deleveraging, Ebbing) => "stabilizing",
        // Clearing = bottoming out
        (Clearing, _) => "stabilizing",
        // Overheating = getting worse
        (Overheating, _) => "worsening",
        _ => "stable",
    }
}

/// Map event risk from VIX + credit cycle phase + game map activity.
fn derive_event_risk(vix: Option<f64>, global_phase: &CreditCyclePhase, max_vector_activity: f64) -> &'static str {
    let vix_risk = match vix {
        Some(v) if v > 40.0 => 4,
        Some(v) if v > 30.0 => 3,
        Some(v) if v > 25.0 => 2,
        Some(v) if v > 20.0 => 1,
        _ => 0,
    };

    let phase_risk = match global_phase {
        CreditCyclePhase::Overheating | CreditCyclePhase::Deleveraging => 2,
        CreditCyclePhase::Tightening | CreditCyclePhase::Clearing => 1,
        _ => 0,
    };

    let geo_risk = if max_vector_activity > 0.7 { 2 } else if max_vector_activity > 0.5 { 1 } else { 0 };

    let total = vix_risk + phase_risk + geo_risk;

    if total >= 6 { "critical" }
    else if total >= 4 { "high" }
    else if total >= 2 { "medium" }
    else if total >= 1 { "low" }
    else { "none" }
}

/// Write a new MarketContext row, composing data from five-layer model.
/// Called periodically by poll_loop to keep QuantTerminal informed.
pub async fn write_market_context(
    mc_pool: &SqlitePool,
    main_pool: &SqlitePool,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();

    // === Gather five-layer data ===

    // VIX level
    let vix: Option<f64> = sqlx::query_scalar(
        "SELECT price FROM market_snap WHERE symbol = '^VIX' ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(main_pool)
    .await
    .ok()
    .flatten();

    // Fear & Greed
    let fear_greed: Option<f64> = sqlx::query_scalar(
        "SELECT value FROM macro_data WHERE indicator = 'fear_greed_index' ORDER BY fetched_at DESC LIMIT 1",
    )
    .fetch_optional(main_pool)
    .await
    .ok()
    .flatten();

    // News sentiment from AI analysis (average confidence of last 24h)
    let news_sentiment: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(AVG(confidence), 0.0) FROM ai_analysis
           WHERE analysis_type = 'news_summary'
             AND created_at >= datetime('now', '-24 hours')"#,
    )
    .fetch_one(main_pool)
    .await
    .unwrap_or(0.0);

    // Map news_sentiment (0-1) to -5 to +5 scale
    let sentiment_mapped = (news_sentiment * 10.0) - 5.0;

    // === Credit cycle layer (single entry point) ===
    let overview = global_aggregator::compute_global_overview(main_pool).await;
    let (global_phase, tide_state, _risk_alerts_count, overview_confidence) = match &overview {
        Ok(o) => (
            o.global_phase.clone(),
            o.dollar_tide.tide_state.clone(),
            o.risk_alerts.len(),
            o.confidence,
        ),
        Err(e) => {
            log::warn!("Credit cycle computation failed, using defaults: {}", e);
            (CreditCyclePhase::Unknown, TideState::Neutral, 0, 0.0)
        }
    };

    // === Game map layer (policy vector activity) ===
    let vectors = game_map::get_policy_vectors(main_pool).await.unwrap_or_default();
    let max_vector_activity = vectors.iter().map(|v| v.activity).fold(0.0_f64, f64::max);

    // === Derive QuantTerminal regime fields ===
    let market_regime = map_cycle_to_regime(&global_phase, vix);
    let regime_trend = derive_regime_trend(&tide_state, &global_phase);
    let event_risk = derive_event_risk(vix, &global_phase, max_vector_activity);

    // Confidence: use global aggregator confidence, bounded
    let regime_confidence = overview_confidence.clamp(0.0, 1.0);

    // Legacy regime string (risk-on / risk-off / neutral)
    let regime = match (vix, fear_greed) {
        (Some(v), Some(fg)) if v > 30.0 || fg < 25.0 => "risk-off",
        (Some(v), Some(fg)) if v < 20.0 && fg > 55.0 => "risk-on",
        _ => "neutral",
    };

    // Sector bias from cycle reasoning
    let sector_bias: Option<String> = sqlx::query_scalar(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'cycle_reasoning'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(main_pool)
    .await
    .ok()
    .flatten()
    .and_then(|json_str: String| {
        serde_json::from_str::<serde_json::Value>(&json_str)
            .ok()
            .and_then(|v| {
                v.get("sectorRecommendations")
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.as_str().map(|s| s.to_string()))
            })
    });

    // === Upcoming events from decision calendar ===
    let calendar = game_map::get_calendar_events(30).unwrap_or_default();
    let upcoming_events: Vec<serde_json::Value> = calendar
        .iter()
        .take(5)
        .map(|evt| {
            serde_json::json!({
                "name": evt.title,
                "scheduledAt": evt.date,
                "impact": evt.impact_direction,
                "affectedSymbols": evt.affected_assets,
            })
        })
        .collect();
    let upcoming_events_json = serde_json::to_string(&upcoming_events).unwrap_or_else(|_| "[]".to_string());

    // === Reasoning summary ===
    let phase_str = global_phase.display_name_zh();
    let tide_str = tide_state.display_name_zh();
    let reasoning = format!(
        "Credit cycle: {} (conf {:.0}%). Dollar tide: {}. Top policy vector activity: {:.0}%. VIX: {:.1}. F&G: {:.0}.",
        phase_str,
        regime_confidence * 100.0,
        tide_str,
        max_vector_activity * 100.0,
        vix.unwrap_or(0.0),
        fear_greed.unwrap_or(0.0),
    );

    let summary = format!(
        "Regime: {} ({}), EventRisk: {}, CreditCycle: {}, DollarTide: {}",
        market_regime, regime, event_risk, phase_str, tide_str,
    );

    // === Insert row ===
    sqlx::query(
        r#"INSERT INTO market_context
           (timestamp, regime, event_risk, vix_level, sector_bias, news_sentiment,
            upcoming_events, summary, source, schema_version,
            market_regime, regime_confidence, regime_reasoning, regime_trend,
            regime_hist_median_days, regime_hist_min_days, regime_hist_max_days)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'zhiku', 2,
                   ?9, ?10, ?11, ?12, NULL, NULL, NULL)"#,
    )
    .bind(&now)
    .bind(regime)
    .bind(event_risk)
    .bind(vix)
    .bind(sector_bias.as_deref())
    .bind(sentiment_mapped)
    .bind(&upcoming_events_json)
    .bind(&summary)
    .bind(market_regime)
    .bind(regime_confidence)
    .bind(&reasoning)
    .bind(regime_trend)
    .execute(mc_pool)
    .await
    .map_err(|e| AppError::Database(format!("market_context insert failed: {}", e)))?;

    // Keep only last 1000 rows
    sqlx::query(
        "DELETE FROM market_context WHERE id NOT IN (SELECT id FROM market_context ORDER BY id DESC LIMIT 1000)",
    )
    .execute(mc_pool)
    .await
    .ok();

    log::info!(
        "MarketContext written: market_regime={}, event_risk={}, cycle={}, tide={}",
        market_regime, event_risk, phase_str, tide_str,
    );
    Ok(())
}
