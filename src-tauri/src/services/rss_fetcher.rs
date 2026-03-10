use chrono::Utc;
use feed_rs::parser;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;

/// RSS source definition with metadata.
struct RssSource {
    url: &'static str,
    name: &'static str,
    tier: i64,      // 1-4 source credibility
    language: &'static str,
}

/// Hardcoded financial RSS feeds for Phase 2 (US vertical slice).
/// Tier 1 = most credible, Tier 4 = least.
const RSS_SOURCES: &[RssSource] = &[
    RssSource {
        url: "https://feeds.reuters.com/reuters/businessNews",
        name: "Reuters Business",
        tier: 1,
        language: "en",
    },
    RssSource {
        url: "https://feeds.bbci.co.uk/news/business/rss.xml",
        name: "BBC Business",
        tier: 1,
        language: "en",
    },
    RssSource {
        url: "https://www.cnbc.com/id/100003114/device/rss/rss.html",
        name: "CNBC",
        tier: 2,
        language: "en",
    },
    RssSource {
        url: "https://feeds.marketwatch.com/marketwatch/topstories",
        name: "MarketWatch",
        tier: 2,
        language: "en",
    },
    RssSource {
        url: "https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=20910258",
        name: "CNBC Finance",
        tier: 2,
        language: "en",
    },
    RssSource {
        url: "https://rss.nytimes.com/services/xml/rss/nyt/Business.xml",
        name: "NYT Business",
        tier: 1,
        language: "en",
    },
];

/// Fetch all configured RSS feeds and insert new articles into the `news` table.
/// Uses `INSERT OR IGNORE` with the `url UNIQUE` constraint for deduplication (ZK-05).
///
/// # Returns
/// Total count of newly inserted articles, or `AppError` on fatal failure.
/// Individual feed failures are logged and skipped (no panic).
pub async fn fetch_all_rss(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("Failed to create HTTP client: {}", e)))?;

    let mut total_inserted: usize = 0;

    for source in RSS_SOURCES {
        match fetch_single_feed(&client, pool, source).await {
            Ok(count) => {
                log::info!("RSS [{}]: {} new articles", source.name, count);
                total_inserted += count;
            }
            Err(e) => {
                log::warn!("RSS [{}] failed: {}", source.name, e);
                // Continue with next feed — do not abort on single failure
            }
        }
    }

    Ok(total_inserted)
}

/// Fetch and parse a single RSS feed, inserting new entries.
async fn fetch_single_feed(
    client: &reqwest::Client,
    pool: &SqlitePool,
    source: &RssSource,
) -> Result<usize, AppError> {
    let response = client
        .get(source.url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .send()
        .await?;

    let bytes = response.bytes().await?;

    let feed = parser::parse(&bytes[..])
        .map_err(|e| AppError::Parse(format!("RSS parse error for {}: {}", source.name, e)))?;

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    for entry in &feed.entries {
        let id = Uuid::new_v4().to_string();

        // Extract the article URL (link)
        let url = match entry.links.first() {
            Some(link) => link.href.clone(),
            None => continue, // Skip entries without a URL
        };

        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| "(no title)".to_string());

        let published_at = entry
            .published
            .or(entry.updated)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| now.clone());

        let snippet = entry
            .summary
            .as_ref()
            .map(|s| s.content.clone())
            .or_else(|| {
                entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.as_ref())
                    .map(|b| {
                        // Strip HTML tags for a clean snippet
                        let text = b.replace("<br>", "\n").replace("<br/>", "\n");
                        strip_html_tags(&text)
                    })
            });

        // INSERT OR IGNORE: if url already exists, row is silently skipped (dedup)
        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO news
               (id, url, title, source, source_tier, category, published_at,
                fetched_at, content_snippet, language, source_url)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
        )
        .bind(&id)
        .bind(&url)
        .bind(&title)
        .bind(source.name)
        .bind(source.tier)
        .bind("market") // Default category; AI will reclassify in Phase 3
        .bind(&published_at)
        .bind(&now)
        .bind(&snippet)
        .bind(source.language)
        .bind(&url) // source_url = original link (ZK-01)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Insert news failed: {}", e)))?;

        if result.rows_affected() > 0 {
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// Simple HTML tag stripper (no external dependency).
fn strip_html_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut inside_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }
    result
}
