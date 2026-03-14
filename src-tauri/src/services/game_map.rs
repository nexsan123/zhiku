use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Policy Vectors — 6 dimensions of US policy
// ---------------------------------------------------------------------------

/// A US policy vector with current activity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyVector {
    /// Vector identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Chinese display name.
    pub name_zh: String,
    /// Current activity level (0.0-1.0) based on recent news density.
    pub activity: f64,
    /// Activity description: "dormant", "low", "moderate", "high", "critical".
    pub activity_label: String,
    /// Assets most affected by this vector.
    pub affected_assets: Vec<String>,
    /// Latest headline related to this vector.
    pub latest_headline: String,
    /// Number of related news in last 7 days.
    pub news_count_7d: i64,
}

/// The 6 policy vector definitions.
const VECTOR_DEFS: &[(&str, &str, &str, &[&str])] = &[
    (
        "trade",
        "Trade Policy",
        "贸易政策",
        &["^GSPC", "000001.SS", "USDCNY=X", "HG=F"],
    ),
    (
        "tech",
        "Tech Controls",
        "科技管制",
        &["^IXIC", "000001.SS"],
    ),
    (
        "financial",
        "Financial/Monetary",
        "金融货币",
        &["^GSPC", "DX-Y.NYB", "GC=F", "BTC-USD", "DGS10"],
    ),
    (
        "energy",
        "Energy Policy",
        "能源政策",
        &["CL=F", "NG=F"],
    ),
    (
        "crypto",
        "Crypto Regulation",
        "加密监管",
        &["BTC-USD", "ETH-USD"],
    ),
    (
        "military",
        "Military/Security",
        "军事安全",
        &["GC=F", "CL=F", "DX-Y.NYB"],
    ),
];

/// Category keywords for matching news to policy vectors.
const VECTOR_KEYWORDS: &[(&str, &[&str])] = &[
    ("trade", &["tariff", "trade war", "section 301", "import duty", "export control", "ustr", "wto dispute"]),
    ("tech", &["chip", "semiconductor", "ai regulation", "entity list", "huawei", "nvidia", "export restriction", "tech ban"]),
    ("financial", &["federal reserve", "fomc", "interest rate", "monetary policy", "treasury", "yield", "debt ceiling", "qe", "qt"]),
    ("energy", &["oil sanction", "opec", "lng", "energy security", "price cap", "pipeline", "renewable", "eia"]),
    ("crypto", &["sec crypto", "bitcoin etf", "stablecoin", "cftc", "digital asset", "crypto regulation", "cbdc"]),
    ("military", &["nato", "defense", "sanction", "arms", "military", "security", "ofac", "war"]),
];

// ---------------------------------------------------------------------------
// Bilateral Dynamics — 4 key relationships
// ---------------------------------------------------------------------------

/// Bilateral relationship state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilateralDynamic {
    /// Relationship identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Chinese display name.
    pub name_zh: String,
    /// Current tension level (0.0-1.0). 0=cooperative, 1=hostile.
    pub tension: f64,
    /// Tension label: "cooperative", "cautious", "strained", "hostile".
    pub tension_label: String,
    /// Recent news headlines related to this relationship.
    pub recent_headlines: Vec<String>,
    /// Number of related news in last 7 days.
    pub news_count_7d: i64,
}

const BILATERAL_DEFS: &[(&str, &str, &str, &[&str])] = &[
    ("us_cn", "US-China", "中美博弈", &["china", "beijing", "xi jinping", "us-china", "sino"]),
    ("us_ru", "US-Russia", "美俄关系", &["russia", "moscow", "putin", "ukraine", "nato"]),
    ("us_me", "US-Middle East", "美国-中东", &["saudi", "iran", "opec", "israel", "gulf"]),
    ("us_eu", "US-Europe", "美欧关系", &["europe", "eu", "brussels", "european", "euro area"]),
];

// ---------------------------------------------------------------------------
// Decision Calendar Event
// ---------------------------------------------------------------------------

/// A policy decision calendar event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub date: String,
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub affected_assets: Vec<String>,
    pub impact_direction: String,
    pub policy_vector: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get all 6 policy vectors with current activity levels.
pub async fn get_policy_vectors(pool: &SqlitePool) -> Result<Vec<PolicyVector>, AppError> {
    let mut vectors = Vec::with_capacity(6);

    for (id, name, name_zh, assets) in VECTOR_DEFS {
        let keywords = VECTOR_KEYWORDS
            .iter()
            .find(|(vid, _)| vid == id)
            .map(|(_, kw)| *kw)
            .unwrap_or(&[]);

        let (news_count, latest_headline) =
            count_vector_news(pool, keywords).await;

        let activity = compute_activity(news_count);
        let activity_label = activity_label(activity);

        vectors.push(PolicyVector {
            id: id.to_string(),
            name: name.to_string(),
            name_zh: name_zh.to_string(),
            activity,
            activity_label,
            affected_assets: assets.iter().map(|s| s.to_string()).collect(),
            latest_headline,
            news_count_7d: news_count,
        });
    }

    Ok(vectors)
}

/// Get all 4 bilateral dynamics with current tension levels.
pub async fn get_bilateral_dynamics(pool: &SqlitePool) -> Result<Vec<BilateralDynamic>, AppError> {
    let mut dynamics = Vec::with_capacity(4);

    for (id, name, name_zh, keywords) in BILATERAL_DEFS {
        let (news_count, headlines) =
            count_bilateral_news(pool, keywords).await;

        let tension = compute_tension(news_count);
        let tension_label = tension_label(tension);

        dynamics.push(BilateralDynamic {
            id: id.to_string(),
            name: name.to_string(),
            name_zh: name_zh.to_string(),
            tension,
            tension_label,
            recent_headlines: headlines,
            news_count_7d: news_count,
        });
    }

    Ok(dynamics)
}

/// Load decision calendar events within the next N days.
pub fn get_calendar_events(days: i64) -> Result<Vec<CalendarEvent>, AppError> {
    let calendar_json = include_str!("../data/policy_calendar.json");
    let parsed: serde_json::Value = serde_json::from_str(calendar_json)
        .map_err(|e| AppError::Parse(format!("Failed to parse policy_calendar.json: {}", e)))?;

    let today = chrono::Utc::now().date_naive();
    let cutoff = today + chrono::Duration::days(days);

    let events = parsed
        .get("events")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let date_str = e.get("date")?.as_str()?;
                    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
                    if date >= today && date <= cutoff {
                        serde_json::from_value::<CalendarEvent>(e.clone()).ok()
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(events)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Count news matching vector keywords in last 7 days + get latest headline.
async fn count_vector_news(pool: &SqlitePool, keywords: &[&str]) -> (i64, String) {
    if keywords.is_empty() {
        return (0, String::new());
    }

    // Build LIKE conditions for each keyword
    let conditions: Vec<String> = keywords
        .iter()
        .map(|kw| format!("(LOWER(title) LIKE '%{}%' OR LOWER(ai_summary) LIKE '%{}%')", kw, kw))
        .collect();

    let where_clause = conditions.join(" OR ");
    let query = format!(
        "SELECT COUNT(*) FROM news WHERE ({}) AND published_at >= datetime('now', '-7 days')",
        where_clause
    );

    let count: i64 = sqlx::query_scalar(&query)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let headline_query = format!(
        "SELECT title FROM news WHERE ({}) AND published_at >= datetime('now', '-7 days') ORDER BY published_at DESC LIMIT 1",
        where_clause
    );

    let headline: Option<String> = sqlx::query_scalar(&headline_query)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    (count, headline.unwrap_or_default())
}

/// Count bilateral relationship news + get top 3 headlines.
async fn count_bilateral_news(pool: &SqlitePool, keywords: &[&str]) -> (i64, Vec<String>) {
    if keywords.is_empty() {
        return (0, Vec::new());
    }

    let conditions: Vec<String> = keywords
        .iter()
        .map(|kw| format!("(LOWER(title) LIKE '%{}%' OR LOWER(ai_summary) LIKE '%{}%')", kw, kw))
        .collect();

    let where_clause = conditions.join(" OR ");
    let query = format!(
        "SELECT COUNT(*) FROM news WHERE ({}) AND published_at >= datetime('now', '-7 days')",
        where_clause
    );

    let count: i64 = sqlx::query_scalar(&query)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let headline_query = format!(
        "SELECT title FROM news WHERE ({}) AND published_at >= datetime('now', '-7 days') ORDER BY published_at DESC LIMIT 3",
        where_clause
    );

    let headlines: Vec<(String,)> = sqlx::query_as(&headline_query)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    (count, headlines.into_iter().map(|(t,)| t).collect())
}

/// Map news count to activity level (0.0-1.0).
fn compute_activity(news_count: i64) -> f64 {
    match news_count {
        0 => 0.0,
        1..=2 => 0.2,
        3..=5 => 0.4,
        6..=10 => 0.6,
        11..=20 => 0.8,
        _ => 1.0,
    }
}

fn activity_label(activity: f64) -> String {
    if activity < 0.1 {
        "dormant"
    } else if activity < 0.3 {
        "low"
    } else if activity < 0.6 {
        "moderate"
    } else if activity < 0.9 {
        "high"
    } else {
        "critical"
    }
    .to_string()
}

/// Map news count to tension level (0.0-1.0).
fn compute_tension(news_count: i64) -> f64 {
    match news_count {
        0 => 0.1, // baseline tension always exists
        1..=3 => 0.3,
        4..=8 => 0.5,
        9..=15 => 0.7,
        _ => 0.9,
    }
}

fn tension_label(tension: f64) -> String {
    if tension < 0.25 {
        "cooperative"
    } else if tension < 0.45 {
        "cautious"
    } else if tension < 0.7 {
        "strained"
    } else {
        "hostile"
    }
    .to_string()
}
