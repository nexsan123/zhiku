use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::news::NewsHeatmapEntry;

/// Country keyword mapping for heatmap aggregation.
/// Each entry: (ISO country code, list of keywords to match in title/ai_summary).
const COUNTRY_KEYWORDS: &[(&str, &[&str])] = &[
    ("US", &["United States", "U.S.", "USA", "美国", "Fed", "FOMC", "White House", "Trump", "Biden"]),
    ("CN", &["China", "中国", "Beijing", "PBoC", "人民银行", "习近平"]),
    ("JP", &["Japan", "日本", "BOJ", "日银", "Tokyo"]),
    ("GB", &["UK", "Britain", "England", "英国", "BOE"]),
    ("XM", &["Euro", "ECB", "欧元", "欧央行", "Eurozone"]),
    ("IN", &["India", "印度", "RBI", "Modi"]),
    ("BR", &["Brazil", "巴西"]),
    ("KR", &["Korea", "韩国", "KOSPI", "Seoul"]),
    ("AU", &["Australia", "澳大利亚", "RBA"]),
    ("CA", &["Canada", "加拿大", "BOC"]),
    ("SA", &["Saudi", "沙特", "OPEC"]),
    ("AE", &["UAE", "Dubai", "阿联酋", "迪拜"]),
    ("TR", &["Turkey", "土耳其", "Ankara"]),
    ("AR", &["Argentina", "阿根廷"]),
    ("ZA", &["South Africa", "南非"]),
    ("RU", &["Russia", "俄罗斯", "Moscow"]),
    ("IR", &["Iran", "伊朗"]),
    ("DE", &["Germany", "德国", "Berlin"]),
    ("FR", &["France", "法国", "Paris"]),
    ("TW", &["Taiwan", "台湾", "TSMC"]),
];

/// English stop words to filter from keyword extraction.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "must",
    "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
    "into", "through", "during", "before", "after", "above", "below",
    "and", "but", "or", "nor", "not", "so", "yet", "both", "either",
    "this", "that", "these", "those", "it", "its", "he", "she", "they",
    "his", "her", "their", "our", "my", "your", "we", "you", "me",
    "said", "says", "say", "new", "also", "than", "more", "just",
    "about", "over", "after", "up", "out", "no", "if", "when", "all",
    "which", "what", "who", "how", "where", "there", "here",
];

/// Minimal row for heatmap aggregation (avoids fetching unnecessary columns).
#[derive(Debug, sqlx::FromRow)]
struct HeatmapRow {
    title: String,
    ai_summary: Option<String>,
    sentiment_score: Option<f64>,
}

/// Accumulator for per-country aggregation.
struct CountryAccum {
    count: u32,
    sentiment_sum: f64,
    sentiment_count: u32,
    latest_title: String,
    word_freq: HashMap<String, u32>,
}

/// Aggregate news by country for the heatmap layer.
///
/// Queries the `news` table for articles published within the last `hours` hours,
/// matches each article against country keywords (case-insensitive, in title + ai_summary),
/// and returns per-country aggregated stats.
pub async fn aggregate_news_heatmap(
    pool: &SqlitePool,
    hours: u32,
) -> Result<Vec<NewsHeatmapEntry>, AppError> {
    let hours_str = format!("-{} hours", hours);

    let rows = sqlx::query_as::<_, HeatmapRow>(
        "SELECT title, ai_summary, sentiment_score \
         FROM news \
         WHERE published_at >= datetime('now', ?1) \
         ORDER BY published_at DESC \
         LIMIT 1000",
    )
    .bind(&hours_str)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Heatmap query failed: {}", e)))?;

    let mut country_map: HashMap<&str, CountryAccum> = HashMap::new();

    for row in &rows {
        // Build searchable text: title + ai_summary, lowercased
        let search_text = {
            let mut text = row.title.to_lowercase();
            if let Some(ref summary) = row.ai_summary {
                text.push(' ');
                text.push_str(&summary.to_lowercase());
            }
            text
        };

        // Match against each country's keywords
        for &(code, keywords) in COUNTRY_KEYWORDS {
            let matched = keywords.iter().any(|kw| {
                search_text.contains(&kw.to_lowercase())
            });

            if matched {
                let accum = country_map.entry(code).or_insert_with(|| CountryAccum {
                    count: 0,
                    sentiment_sum: 0.0,
                    sentiment_count: 0,
                    latest_title: row.title.clone(),
                    word_freq: HashMap::new(),
                });

                accum.count += 1;

                if let Some(score) = row.sentiment_score {
                    accum.sentiment_sum += score;
                    accum.sentiment_count += 1;
                }

                // Extract keywords from title
                extract_keywords(&row.title, &mut accum.word_freq);
            }
        }
    }

    // Convert accumulators to output entries
    let mut result: Vec<NewsHeatmapEntry> = country_map
        .into_iter()
        .map(|(code, accum)| {
            let avg_sentiment = if accum.sentiment_count > 0 {
                accum.sentiment_sum / accum.sentiment_count as f64
            } else {
                0.5 // neutral default when no sentiment data
            };

            // Top 3 keywords by frequency
            let mut word_vec: Vec<(&String, &u32)> = accum.word_freq.iter().collect();
            word_vec.sort_by(|a, b| b.1.cmp(a.1));
            let top_keywords: Vec<String> = word_vec
                .into_iter()
                .take(3)
                .map(|(w, _)| w.clone())
                .collect();

            NewsHeatmapEntry {
                country_code: code.to_string(),
                news_count: accum.count,
                avg_sentiment,
                top_keywords,
                latest_title: accum.latest_title,
            }
        })
        .collect();

    // Sort by news_count descending for consistent output
    result.sort_by(|a, b| b.news_count.cmp(&a.news_count));

    Ok(result)
}

/// Extract meaningful keywords from text into frequency map.
/// Filters stop words and short tokens (<3 chars).
fn extract_keywords(text: &str, freq: &mut HashMap<String, u32>) {
    for word in text.split_whitespace() {
        // Strip punctuation from edges
        let clean: String = word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c > '\u{7F}') // keep CJK chars
            .collect();

        if clean.is_empty() {
            continue;
        }

        let lower = clean.to_lowercase();

        // Skip short ASCII words and stop words
        let is_ascii_word = lower.chars().all(|c| c.is_ascii_alphabetic());
        if is_ascii_word && (lower.len() < 3 || STOP_WORDS.contains(&lower.as_str())) {
            continue;
        }

        // Skip pure numbers
        if lower.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        *freq.entry(lower).or_insert(0) += 1;
    }
}
