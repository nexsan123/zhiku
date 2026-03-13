use chrono::Utc;
use feed_rs::parser;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;

/// RSS source definition with metadata.
/// `is_rsshub`: if true, `url` contains only the route path (e.g. "/caixin/latest")
/// and the full URL is built at runtime as `{rsshub_base_url}{route}`.
pub struct RssSource {
    pub url: &'static str,
    pub name: &'static str,
    pub tier: i64,      // 0-3 source credibility (T0=govt, T1=wire, T2=media, T3=niche)
    pub language: &'static str,
    pub is_rsshub: bool,
}

/// Financial RSS feeds for global intelligence coverage.
/// Tier 0 = government/institutional data (央行, 统计局, 监管机构 RSS feeds)
/// Tier 1 = national wire services (AP, Xinhua, BBC)
/// Tier 2 = independent financial media (FT, WSJ, CNBC, 财新)
/// Tier 3 = specialized/niche sources (CoinDesk, ZeroHedge, OilPrice)
/// AI reasoning weight: T0=1.0, T1=0.8, T2=0.6, T3=0.3
/// Sources marked "TODO: verify RSS availability" may not have working RSS feeds.
///
/// RSSHub sources: `url` stores the route path only; base URL is configurable
/// via `rsshub_base_url` setting (default: `https://rsshub.app`).
pub const RSS_SOURCES: &[RssSource] = &[
    // =========================================================================
    // ENGLISH: Wire Services & Top-Tier Journalism (Tier 1)
    // =========================================================================
    // Reuters Business — REMOVED: feeds.reuters.com DNS dead
    // Reuters Top News — REMOVED: feeds.reuters.com DNS dead
    RssSource {
        url: "https://feeds.bbci.co.uk/news/business/rss.xml",
        name: "BBC Business",
        tier: 1,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://rss.nytimes.com/services/xml/rss/nyt/Business.xml",
        name: "NYT Business",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://rss.nytimes.com/services/xml/rss/nyt/Economy.xml",
        name: "NYT Economy",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "/apnews/topics/apf-business", // RSSHub proxy for AP News
        name: "AP News",
        tier: 1,
        language: "en",
        is_rsshub: true,
    },
    RssSource {
        url: "https://www.ft.com/?format=rss", // TODO: verify RSS availability — FT may require login
        name: "Financial Times",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://feeds.content.dowjones.io/public/rss/mw_topstories", // TODO: verify RSS availability — WSJ paywall
        name: "WSJ Top Stories",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.economist.com/finance-and-economics/rss.xml",
        name: "The Economist Finance",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },

    // =========================================================================
    // ENGLISH: Major Financial Media (Tier 2)
    // =========================================================================
    RssSource {
        url: "https://www.cnbc.com/id/100003114/device/rss/rss.html",
        name: "CNBC Top News",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=20910258",
        name: "CNBC Finance",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.cnbc.com/id/100727362/device/rss/rss.html",
        name: "CNBC World",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://feeds.marketwatch.com/marketwatch/topstories",
        name: "MarketWatch Top",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://feeds.marketwatch.com/marketwatch/marketpulse",
        name: "MarketWatch Pulse",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.forbes.com/business/feed/",
        name: "Forbes Business",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    // Forbes Markets — REMOVED: Datadome blocks all requests, Forbes Business already covers
    RssSource {
        url: "https://fortune.com/feed/",
        name: "Fortune",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://feeds.bloomberg.com/markets/news.rss", // TODO: verify RSS availability — Bloomberg frequently changes/disables RSS
        name: "Bloomberg Markets",
        tier: 1,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.businessinsider.com/rss",
        name: "Business Insider",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://rss.cnn.com/rss/money_news_economy.rss",
        name: "CNN Business Economy",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://rss.cnn.com/rss/money_markets.rss",
        name: "CNN Markets",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.theguardian.com/business/rss",
        name: "The Guardian Business",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.aljazeera.com/xml/rss/all.xml",
        name: "Al Jazeera",
        tier: 1,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://feeds.finance.yahoo.com/rss/2.0/headline?s=^GSPC&region=US&lang=en-US", // TODO: verify RSS availability — Yahoo Finance RSS is unstable
        name: "Yahoo Finance",
        tier: 2,
        language: "en",
        is_rsshub: false,
    },

    // =========================================================================
    // ENGLISH: Specialized & Crypto (Tier 3 — Niche)
    // =========================================================================
    RssSource {
        url: "https://seekingalpha.com/market_currents.xml",
        name: "Seeking Alpha Currents",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.investing.com/rss/news.rss", // TODO: verify RSS availability
        name: "Investing.com",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    // Barron's — REMOVED: feeds.barrons.com DNS dead / paywall
    RssSource {
        url: "https://feeds.feedburner.com/zerohedge/feed",
        name: "ZeroHedge",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://cointelegraph.com/rss",
        name: "CoinTelegraph",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://coindesk.com/arc/outboundfeeds/rss/",
        name: "CoinDesk",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.theblock.co/rss.xml", // TODO: verify RSS availability
        name: "The Block",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://oilprice.com/rss/main",
        name: "OilPrice.com",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.mining.com/feed/",
        name: "Mining.com",
        tier: 3,
        language: "en",
        is_rsshub: false,
    },

    // =========================================================================
    // CHINESE: Major Financial Media (Tier 1-2)
    // =========================================================================
    RssSource {
        url: "https://finance.sina.com.cn/roll/cj/rss/cj_hot.xml", // TODO: verify RSS availability — Sina may have discontinued RSS
        name: "Sina Finance Hot",
        tier: 2,
        language: "zh",
        is_rsshub: false,
    },
    RssSource {
        url: "/caixin/latest", // RSSHub proxy for Caixin
        name: "Caixin Latest",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/wallstreetcn/news/global", // RSSHub proxy for WallStreetCN
        name: "WallStreetCN Global",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/yicai/headline", // RSSHub proxy for Yicai
        name: "Yicai Headlines",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/cls/telegraph", // RSSHub proxy for CLS (财联社)
        name: "CLS Telegraph",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/eastmoney/report/strategy", // RSSHub proxy for EastMoney
        name: "EastMoney Strategy",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "https://www.ftchinese.com/rss/feed", // FT Chinese official RSS
        name: "FT Chinese",
        tier: 2,
        language: "zh",
        is_rsshub: false,
    },
    RssSource {
        url: "/ifeng/feng/finance", // RSSHub proxy for iFeng Finance
        name: "iFeng Finance",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/163/money/special", // RSSHub proxy for NetEase Finance
        name: "NetEase Finance",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/21caijing/channel/finance", // RSSHub proxy for 21st Century Business Herald
        name: "21st Century Biz Herald",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/eeo/15", // RSSHub proxy for Economic Observer (经济观察报)
        name: "Economic Observer",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/stcn/kuaixun", // RSSHub proxy for Securities Times (证券时报)
        name: "Securities Times Express",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/cs/news/rolling", // RSSHub proxy for China Securities Journal (中国证券报)
        name: "China Securities Journal",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/cnstock/ssnews", // RSSHub proxy for Shanghai Securities News (上海证券报)
        name: "Shanghai Securities News",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/nbd/daily", // RSSHub proxy for National Business Daily (每日经济新闻)
        name: "National Business Daily",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/jiemian/list/4", // RSSHub proxy for Jiemian Finance (界面新闻财经)
        name: "Jiemian Finance",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/thepaper/channel/25950", // RSSHub proxy for The Paper Finance (澎湃新闻财经)
        name: "The Paper Finance",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "http://www.news.cn/feed/finance.xml", // TODO: verify RSS availability — Xinhua Finance (新华社财经)
        name: "Xinhua Finance",
        tier: 1,
        language: "zh",
        is_rsshub: false,
    },
    RssSource {
        url: "http://www.people.com.cn/rss/finance.xml", // People's Daily Finance direct feed (人民网财经)
        name: "People's Daily Finance",
        tier: 1,
        language: "zh",
        is_rsshub: false,
    },
    RssSource {
        url: "/ce/macro", // RSSHub proxy for China Economy Net (中国经济网)
        name: "China Economy Net",
        tier: 2,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/huanqiu/finance", // RSSHub proxy for Global Times Finance (环球时报财经)
        name: "Global Times Finance",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/gelonghui/home", // RSSHub proxy for Gelonghui (格隆汇)
        name: "Gelonghui",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/jin10", // RSSHub proxy for Jin10 (金十数据)
        name: "Jin10 Data",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    RssSource {
        url: "/zhitongcaijing/recommend", // RSSHub proxy for Zhitong Finance (智通财经)
        name: "Zhitong Finance",
        tier: 3,
        language: "zh",
        is_rsshub: true,
    },
    // =========================================================================
    // US POLICY SOURCES (Tier 0 — Government/Institutional) — Game Map / Scenario Engine input
    // =========================================================================
    RssSource {
        url: "https://www.whitehouse.gov/presidential-actions/feed/",
        name: "White House",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.state.gov/rss-feed/press-releases/feed/",
        name: "State Dept Press",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.sec.gov/news/pressreleases.rss",
        name: "SEC Press Releases",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.cftc.gov/RSS/RSSGP/rssgp.xml",
        name: "CFTC Press",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://www.federalreserve.gov/feeds/press_all.xml",
        name: "Federal Reserve Press",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
    RssSource {
        url: "https://ustr.gov/rss.xml",
        name: "USTR Trade",
        tier: 0,
        language: "en",
        is_rsshub: false,
    },
];

/// Default RSSHub base URL used when no custom URL is configured.
const DEFAULT_RSSHUB_BASE: &str = "https://rsshub.app";

/// Fetch all configured RSS feeds and insert new articles into the `news` table.
/// Uses `INSERT OR IGNORE` with the `url UNIQUE` constraint for deduplication (ZK-05).
///
/// For sources with `is_rsshub: true`, the full URL is built as
/// `{rsshub_base_url}{source.url}` where `source.url` contains the route path.
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `rsshub_base_url` - Base URL for RSSHub instance (e.g. "https://rsshub.app")
///
/// # Returns
/// Total count of newly inserted articles, or `AppError` on fatal failure.
/// Individual feed failures are logged and skipped (no panic).
pub async fn fetch_all_rss(pool: &SqlitePool, rsshub_base_url: &str) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Network(format!("Failed to create HTTP client: {}", e)))?;

    // Normalize base URL: strip trailing slash to avoid double-slash with route paths
    let base = rsshub_base_url.trim_end_matches('/');

    let mut total_inserted: usize = 0;

    for source in RSS_SOURCES {
        // Build the actual fetch URL: RSSHub sources get base+route, others use url directly
        let actual_url = if source.is_rsshub {
            format!("{}{}", base, source.url)
        } else {
            source.url.to_string()
        };

        match fetch_single_feed(&client, pool, source, &actual_url).await {
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

/// Resolve the effective RSSHub base URL from a raw store value.
/// Returns the configured URL if non-empty, otherwise the default.
pub fn resolve_rsshub_base(store_value: &str) -> &str {
    if store_value.is_empty() {
        DEFAULT_RSSHUB_BASE
    } else {
        store_value
    }
}

/// Fetch and parse a single RSS feed, inserting new entries.
async fn fetch_single_feed(
    client: &reqwest::Client,
    pool: &SqlitePool,
    source: &RssSource,
    actual_url: &str,
) -> Result<usize, AppError> {
    let response = client
        .get(actual_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
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
