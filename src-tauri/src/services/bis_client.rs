use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashSet;

use crate::errors::AppError;

/// BIS Statistics Warehouse REST API base (SDMX REST v1).
/// Free, no API key required.
/// Docs: https://stats.bis.org/
/// Note: v2 (data.bis.org) is defunct (404). Migrated to v1 (stats.bis.org) 2026-03.

/// Central bank policy rates (monthly).
const BIS_CBPOL_URL: &str =
    "https://stats.bis.org/api/v1/data/WS_CBPOL/M..?detail=dataonly&format=csv";

/// Credit to non-financial sector (% of GDP, quarterly).
/// Dataset renamed: WS_CREDIT (v2) -> WS_TC (v1).
/// Key filter: Q..P.A.M.770.A (P=private non-fin, 770=% of GDP).
const BIS_CREDIT_URL: &str =
    "https://stats.bis.org/api/v1/data/WS_TC/Q..P.A.M.770.A?detail=dataonly&format=csv";

/// Credit-to-GDP gap (percentage points, quarterly). Basel III early warning.
/// Key filter: Q..P.A.C (P=private non-fin, C=credit-to-GDP gap).
const BIS_CREDIT_GAP_URL: &str =
    "https://stats.bis.org/api/v1/data/WS_CREDIT_GAP/Q..P.A.C?detail=dataonly&format=csv";

/// Debt service ratio for private non-financial sector (%, quarterly).
/// Key filter: Q..P (P=private non-fin sector).
const BIS_DSR_URL: &str =
    "https://stats.bis.org/api/v1/data/WS_DSR/Q..P?detail=dataonly&format=csv";

/// Selected property prices (nominal, YoY %, quarterly).
/// Kuznets cycle reference for real estate-driven credit risk.
const BIS_SPP_URL: &str =
    "https://stats.bis.org/api/v1/data/WS_SPP/Q..N.628?detail=dataonly&format=csv";

/// Target countries for edict-004 (15 countries/areas, 3 tiers).
/// (BIS ref_area code, human label).
const COUNTRIES: &[(&str, &str)] = &[
    // Core tier
    ("US", "United States"),
    ("XM", "Euro Area"),
    ("JP", "Japan"),
    ("CN", "China"),
    // Important tier
    ("GB", "United Kingdom"),
    ("CA", "Canada"),
    ("AU", "Australia"),
    ("KR", "South Korea"),
    ("IN", "India"),
    ("BR", "Brazil"),
    // Monitor tier
    ("TR", "Turkey"),
    ("AR", "Argentina"),
    ("ZA", "South Africa"),
    ("SA", "Saudi Arabia"),
    ("AE", "UAE"),
];

// ---------------------------------------------------------------------------
// Generic BIS dataset fetcher
// ---------------------------------------------------------------------------

/// Fetch a BIS SDMX CSV dataset and insert into `macro_data`.
///
/// Generic over any BIS dataset that returns REF_AREA (or BORROWERS_CTY), TIME_PERIOD, OBS_VALUE.
/// Indicator stored as `{prefix}_{ref_area}` (e.g. "BIS_CBPOL_US").
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `url` - Full BIS SDMX REST API URL
/// * `indicator_prefix` - Prefix for indicator name (e.g. "BIS_CBPOL")
/// * `dataset_label` - Human label for logging (e.g. "CBPOL")
async fn fetch_bis_dataset(
    pool: &SqlitePool,
    url: &str,
    indicator_prefix: &str,
    dataset_label: &str,
) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let response = client
        .get(url)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .header("Accept", "text/csv")
        .send()
        .await
        .map_err(|e| {
            AppError::Network(format!("BIS {} request failed: {}", dataset_label, e))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "BIS {} API returned status {}",
            dataset_label,
            response.status()
        )));
    }

    let body = response.text().await.map_err(|e| {
        AppError::Parse(format!("BIS {} response body read error: {}", dataset_label, e))
    })?;

    let rows = parse_bis_csv(&body);
    if rows.is_empty() {
        log::warn!(
            "BIS {}: 0 valid rows parsed from response ({} bytes)",
            dataset_label,
            body.len()
        );
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    let target_codes: HashSet<&str> =
        COUNTRIES.iter().map(|(code, _)| *code).collect();

    for (ref_area, period, value) in &rows {
        if !target_codes.contains(ref_area.as_str()) {
            continue;
        }

        let indicator = format!("{}_{}", indicator_prefix, ref_area);

        // Dedup: skip if (indicator, period) already exists
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
        )
        .bind(&indicator)
        .bind(period)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!(
                "Check existing BIS {} data failed: {}",
                dataset_label, e
            ))
        })?;

        if exists > 0 {
            continue;
        }

        sqlx::query(
            r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
               VALUES (?1, ?2, ?3, 'bis', ?4)"#,
        )
        .bind(&indicator)
        .bind(*value)
        .bind(period)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Insert BIS {} data failed: {}", dataset_label, e))
        })?;

        inserted += 1;
    }

    log::info!("BIS {}: {} new observations inserted", dataset_label, inserted);
    Ok(inserted)
}

// ---------------------------------------------------------------------------
// Public fetchers for each BIS dataset
// ---------------------------------------------------------------------------

/// Fetch BIS central bank policy rates (WS_CBPOL, monthly).
pub async fn fetch_bis_rates(pool: &SqlitePool) -> Result<usize, AppError> {
    fetch_bis_dataset(pool, BIS_CBPOL_URL, "BIS_CBPOL", "CBPOL").await
}

/// Fetch BIS credit to non-financial sector as % of GDP (WS_TC, quarterly).
/// Dataset renamed from WS_CREDIT (v2) to WS_TC (v1).
/// Indicator: BIS_CREDIT_{country_code}
pub async fn fetch_bis_credit(pool: &SqlitePool) -> Result<usize, AppError> {
    fetch_bis_dataset(pool, BIS_CREDIT_URL, "BIS_CREDIT", "CREDIT").await
}

/// Fetch BIS credit-to-GDP gap (WS_CREDIT_GAP, quarterly).
/// Basel III early warning indicator. >+10pp = danger zone.
/// Indicator: BIS_CREDIT_GAP_{country_code}
pub async fn fetch_bis_credit_gap(pool: &SqlitePool) -> Result<usize, AppError> {
    fetch_bis_dataset(pool, BIS_CREDIT_GAP_URL, "BIS_CREDIT_GAP", "CREDIT_GAP").await
}

/// Fetch BIS debt service ratio (WS_DSR, quarterly).
/// Indicator: BIS_DSR_{country_code}
pub async fn fetch_bis_dsr(pool: &SqlitePool) -> Result<usize, AppError> {
    fetch_bis_dataset(pool, BIS_DSR_URL, "BIS_DSR", "DSR").await
}

/// Fetch BIS selected property prices (WS_SPP, quarterly, nominal YoY %).
/// Indicator: BIS_SPP_{country_code}
pub async fn fetch_bis_spp(pool: &SqlitePool) -> Result<usize, AppError> {
    fetch_bis_dataset(pool, BIS_SPP_URL, "BIS_SPP", "SPP").await
}

/// Fetch all BIS credit-related datasets (CREDIT + CREDIT_GAP + DSR + SPP).
/// Returns total newly inserted observations across all datasets.
pub async fn fetch_all_credit_data(pool: &SqlitePool) -> Result<usize, AppError> {
    let mut total = 0;

    match fetch_bis_credit(pool).await {
        Ok(n) => total += n,
        Err(e) => log::warn!("BIS CREDIT fetch failed: {}", e),
    }
    match fetch_bis_credit_gap(pool).await {
        Ok(n) => total += n,
        Err(e) => log::warn!("BIS CREDIT_GAP fetch failed: {}", e),
    }
    match fetch_bis_dsr(pool).await {
        Ok(n) => total += n,
        Err(e) => log::warn!("BIS DSR fetch failed: {}", e),
    }
    match fetch_bis_spp(pool).await {
        Ok(n) => total += n,
        Err(e) => log::warn!("BIS SPP fetch failed: {}", e),
    }

    log::info!("BIS credit data: {} total new observations", total);
    Ok(total)
}

/// Parse BIS SDMX-CSV response into (country_code, period, value) tuples.
///
/// BIS v1 CSV headers vary by dataset:
///   - CBPOL, SPP use `REF_AREA` for country code
///   - WS_TC, WS_CREDIT_GAP, WS_DSR use `BORROWERS_CTY` for country code
/// We try `REF_AREA` first, then fall back to `BORROWERS_CTY`.
/// TIME_PERIOD and OBS_VALUE are present in all datasets.
fn parse_bis_csv(body: &str) -> Vec<(String, String, f64)> {
    let mut results = Vec::new();
    let mut lines = body.lines();

    // Find header line
    let header = match lines.next() {
        Some(h) => h,
        None => return results,
    };

    let headers: Vec<&str> = header.split(',').collect();
    // Country code column: REF_AREA (CBPOL, SPP) or BORROWERS_CTY (WS_TC, CREDIT_GAP, DSR)
    let ref_area_idx = headers
        .iter()
        .position(|h| h.trim() == "REF_AREA")
        .or_else(|| headers.iter().position(|h| h.trim() == "BORROWERS_CTY"));
    let period_idx = headers
        .iter()
        .position(|h| h.trim() == "TIME_PERIOD");
    let value_idx = headers
        .iter()
        .position(|h| h.trim() == "OBS_VALUE");

    let (ref_area_idx, period_idx, value_idx) =
        match (ref_area_idx, period_idx, value_idx) {
            (Some(a), Some(p), Some(v)) => (a, p, v),
            _ => {
                log::warn!(
                    "BIS CSV header missing expected columns. Headers: {:?}",
                    headers
                );
                return results;
            }
        };

    for line in lines {
        let cols: Vec<&str> = line.split(',').collect();
        let max_idx = *[ref_area_idx, period_idx, value_idx]
            .iter()
            .max()
            .unwrap_or(&0);
        if cols.len() <= max_idx {
            continue;
        }

        let ref_area = cols[ref_area_idx].trim().to_string();
        let period = cols[period_idx].trim().to_string();
        let value_str = cols[value_idx].trim();

        if let Ok(value) = value_str.parse::<f64>() {
            results.push((ref_area, period, value));
        }
    }

    results
}
