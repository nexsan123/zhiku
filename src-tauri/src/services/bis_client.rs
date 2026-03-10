use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::AppError;

/// BIS Data Portal REST API for central bank policy rates (CBPOL).
/// Free, no API key required.
/// Docs: https://data.bis.org/topics/CBPOL
const BIS_CBPOL_URL: &str =
    "https://data.bis.org/api/v2/data/WS_CBPOL/M..?detail=dataonly&format=csv";

/// Target countries: (BIS ref_area code, human label).
const COUNTRIES: &[(&str, &str)] = &[
    ("US", "United States"),
    ("XM", "Euro Area"),
    ("JP", "Japan"),
    ("GB", "United Kingdom"),
    ("CN", "China"),
    ("IN", "India"),
    ("CA", "Canada"),
    ("AU", "Australia"),
    ("CH", "Switzerland"),
    ("KR", "South Korea"),
    ("BR", "Brazil"),
    ("SA", "Saudi Arabia"),
    ("AE", "UAE"),
];

/// Fetch BIS central bank policy rates and insert into `macro_data`.
///
/// Requests CSV format from the BIS SDMX REST API.
/// Each row maps to indicator `BIS_CBPOL_{country_code}` in `macro_data`.
/// Deduplicates by (indicator, period) combination.
///
/// # Returns
/// Count of newly inserted data points, or `AppError` on failure.
pub async fn fetch_bis_rates(pool: &SqlitePool) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    let response = client
        .get(BIS_CBPOL_URL)
        .header("User-Agent", "ZhiKu/0.1 (Financial Intelligence)")
        .header("Accept", "text/csv")
        .send()
        .await
        .map_err(|e| AppError::Network(format!("BIS request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "BIS API returned status {}",
            response.status()
        )));
    }

    let body = response.text().await.map_err(|e| {
        AppError::Parse(format!("BIS response body read error: {}", e))
    })?;

    let rows = parse_bis_csv(&body);
    if rows.is_empty() {
        log::warn!("BIS: 0 valid rates parsed from response ({} bytes)", body.len());
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    // Build a set of target country codes for fast lookup
    let target_codes: std::collections::HashSet<&str> =
        COUNTRIES.iter().map(|(code, _)| *code).collect();

    for (ref_area, period, value) in &rows {
        if !target_codes.contains(ref_area.as_str()) {
            continue;
        }

        let indicator = format!("BIS_CBPOL_{}", ref_area);

        // Dedup: skip if (indicator, period) already exists
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
        )
        .bind(&indicator)
        .bind(period)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            AppError::Database(format!("Check existing BIS data failed: {}", e))
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
            AppError::Database(format!("Insert BIS data failed: {}", e))
        })?;

        inserted += 1;
    }

    log::info!("BIS CBPOL: {} new rate observations inserted", inserted);
    Ok(inserted)
}

/// Parse BIS SDMX-CSV response into (ref_area, period, value) tuples.
///
/// BIS CSV typically has headers like:
///   REF_AREA,TIME_PERIOD,OBS_VALUE,...
/// We find the column indices dynamically for robustness.
fn parse_bis_csv(body: &str) -> Vec<(String, String, f64)> {
    let mut results = Vec::new();
    let mut lines = body.lines();

    // Find header line
    let header = match lines.next() {
        Some(h) => h,
        None => return results,
    };

    let headers: Vec<&str> = header.split(',').collect();
    let ref_area_idx = headers.iter().position(|h| h.trim() == "REF_AREA");
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
