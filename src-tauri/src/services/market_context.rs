use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;

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

    // Create table (idempotent with IF NOT EXISTS per RT-002)
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
            schema_version INTEGER NOT NULL DEFAULT 1
        )"#,
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(format!("market_context schema creation failed: {}", e)))?;

    log::info!("market_context.db initialized at {}", path.display());
    Ok(pool)
}

/// Write a new MarketContext row, composing data from the main DB.
/// Called periodically by poll_loop to keep QuantTerminal informed.
pub async fn write_market_context(
    mc_pool: &SqlitePool,
    main_pool: &SqlitePool,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();

    // VIX level
    let vix: Option<f64> = sqlx::query_scalar(
        "SELECT price FROM market_snap WHERE symbol = '^VIX' ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(main_pool)
    .await
    .ok()
    .flatten();

    // Fear & Greed for sentiment
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

    // Determine regime from VIX + sentiment
    let regime = match (vix, fear_greed) {
        (Some(v), Some(fg)) if v > 30.0 || fg < 25.0 => "risk-off",
        (Some(v), Some(fg)) if v < 20.0 && fg > 55.0 => "risk-on",
        _ => "neutral",
    };

    // Event risk based on VIX
    let event_risk = match vix {
        Some(v) if v > 40.0 => "critical",
        Some(v) if v > 30.0 => "high",
        Some(v) if v > 25.0 => "medium",
        Some(v) if v > 20.0 => "low",
        _ => "none",
    };

    // Latest cycle reasoning for sector bias
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

    // Summary
    let summary = format!(
        "Regime: {}, VIX: {:.1}, Sentiment: {:.1}",
        regime,
        vix.unwrap_or(0.0),
        sentiment_mapped
    );

    // Insert row
    sqlx::query(
        r#"INSERT INTO market_context
           (timestamp, regime, event_risk, vix_level, sector_bias, news_sentiment,
            upcoming_events, summary, source, schema_version)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', ?7, 'zhiku', 1)"#,
    )
    .bind(&now)
    .bind(regime)
    .bind(event_risk)
    .bind(vix)
    .bind(sector_bias.as_deref())
    .bind(sentiment_mapped)
    .bind(&summary)
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
        "MarketContext written: regime={}, event_risk={}",
        regime,
        event_risk
    );
    Ok(())
}
