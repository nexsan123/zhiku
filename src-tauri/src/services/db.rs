use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::str::FromStr;

use crate::errors::AppError;

/// SQL migration: creates all 7 core tables.
/// All statements use IF NOT EXISTS for idempotency (RT-002).
const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS news (
    id TEXT PRIMARY KEY,
    url TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    source TEXT NOT NULL,
    source_tier INTEGER DEFAULT 3,
    category TEXT,
    published_at TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    content_snippet TEXT,
    language TEXT DEFAULT 'en',
    sentiment_score REAL,
    ai_summary TEXT,
    source_url TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS market_snap (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    price REAL NOT NULL,
    change_pct REAL,
    volume REAL,
    timestamp TEXT NOT NULL,
    source TEXT DEFAULT 'yahoo'
);

CREATE TABLE IF NOT EXISTS macro_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    indicator TEXT NOT NULL,
    value REAL NOT NULL,
    period TEXT,
    source TEXT NOT NULL,
    fetched_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_analysis (
    id TEXT PRIMARY KEY,
    analysis_type TEXT NOT NULL,
    input_ids TEXT,
    output TEXT NOT NULL,
    model TEXT NOT NULL,
    confidence REAL,
    reasoning_chain TEXT,
    source_urls TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS signals (
    id TEXT PRIMARY KEY,
    signal_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT,
    data TEXT,
    source_urls TEXT,
    ai_confidence REAL,
    ai_model TEXT,
    created_at TEXT NOT NULL,
    pushed_to_qt INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS api_status (
    service TEXT PRIMARY KEY,
    status TEXT DEFAULT 'idle',
    last_check TEXT,
    last_error TEXT,
    response_ms INTEGER
);

CREATE TABLE IF NOT EXISTS reasoning_scorecard (
    id TEXT PRIMARY KEY,
    reasoning_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    predicted_direction TEXT NOT NULL DEFAULT 'neutral',
    predicted_signals TEXT DEFAULT '[]',
    predicted_sectors TEXT DEFAULT '[]',
    predicted_tail_risks TEXT DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0.0,
    contradictions TEXT DEFAULT '[]',
    contradiction_count INTEGER DEFAULT 0,
    actual_sp500_7d REAL,
    actual_sp500_30d REAL,
    actual_vix_7d REAL,
    actual_sector_perf TEXT,
    direction_correct_7d INTEGER,
    direction_correct_30d INTEGER,
    human_verdict TEXT,
    human_note TEXT,
    reviewed_at TEXT
);
"#;

/// Initialize the SQLite database: create pool, run migrations.
///
/// # Arguments
/// * `db_path` - Absolute path to the SQLite database file.
///
/// # Returns
/// A connected `SqlitePool` ready for use, or `AppError::Database` on failure.
pub async fn init_database(db_path: PathBuf) -> Result<SqlitePool, AppError> {
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::Database(format!("Invalid DB path: {}", e)))?;

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to SQLite: {}", e)))?;

    // Run migrations: execute each statement separately since sqlx
    // does not support multiple statements in one execute call.
    for statement in MIGRATION_SQL.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }
        sqlx::query(trimmed)
            .execute(&pool)
            .await
            .map_err(|e| AppError::Database(format!("Migration failed: {}", e)))?;
    }

    log::info!("Database initialized at: {}", db_path.display());
    Ok(pool)
}

/// Delete market_snap rows older than `retain_hours` hours.
///
/// Uses `julianday()` comparison for reliable SQLite date arithmetic (BUG-010).
/// Called periodically from poll_loop to prevent unbounded table growth.
///
/// # Returns
/// Number of rows deleted, or `AppError::Database` on failure.
pub async fn cleanup_old_market_snaps(pool: &SqlitePool, retain_hours: i64) -> Result<u64, AppError> {
    let retain_days: f64 = retain_hours as f64 / 24.0;

    let result = sqlx::query(
        "DELETE FROM market_snap WHERE julianday('now') - julianday(timestamp) > ?1",
    )
    .bind(retain_days)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Cleanup market_snap failed: {}", e)))?;

    Ok(result.rows_affected())
}
