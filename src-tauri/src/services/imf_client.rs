use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::errors::AppError;

/// IMF DataMapper API base URL.
/// Free, no API key required.
/// Docs: https://www.imf.org/external/datamapper/api/help
const IMF_BASE_URL: &str = "https://www.imf.org/external/datamapper/api/v1";

/// IMF WEO indicators for national balance sheet analysis (edict-005).
const IMF_INDICATORS: &[(&str, &str)] = &[
    ("NGDP_RPCH", "Real GDP Growth"),
    ("GGXCNL_NGDP", "Fiscal Balance"),
    ("BCA_NGDPD", "Current Account Balance"),
    ("GGXWDG_NGDP", "Government Gross Debt"),
    ("GGR_G01_GDP_PT", "Government Revenue"),
];

/// Target countries: (IMF 3-letter code, our 2-letter code for indicator naming).
const IMF_COUNTRIES: &[(&str, &str)] = &[
    ("USA", "US"),
    ("CHN", "CN"),
    ("EURO", "XM"),
    ("JPN", "JP"),
    ("GBR", "GB"),
    ("CAN", "CA"),
    ("AUS", "AU"),
    ("KOR", "KR"),
    ("IND", "IN"),
    ("BRA", "BR"),
    ("TUR", "TR"),
    ("ARG", "AR"),
    ("ZAF", "ZA"),
    ("SAU", "SA"),
    ("ARE", "AE"),
];

/// IMF DataMapper JSON response structure.
/// `values.{indicator}.{country_code}.{year}` = value or null
#[derive(Debug, serde::Deserialize)]
struct ImfResponse {
    values: HashMap<String, HashMap<String, HashMap<String, Option<f64>>>>,
}

/// Fetch a single IMF indicator for all 15 countries.
async fn fetch_indicator(
    pool: &SqlitePool,
    indicator_code: &str,
    label: &str,
) -> Result<usize, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("curl/8.4.0")
        .build()
        .map_err(|e| AppError::Network(format!("HTTP client error: {}", e)))?;

    // Build country path segment: USA/CHN/EURO/JPN/...
    let countries_path: String = IMF_COUNTRIES
        .iter()
        .map(|(imf_code, _)| *imf_code)
        .collect::<Vec<_>>()
        .join("/");

    // Query last 4 years + next 2 years of forecasts
    let current_year = Utc::now()
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap_or(2026);
    let periods: String = ((current_year - 3)..=(current_year + 2))
        .map(|y| y.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let url = format!(
        "{}/{}/{}?periods={}",
        IMF_BASE_URL, indicator_code, countries_path, periods
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            AppError::Network(format!("IMF request failed for {}: {}", indicator_code, e))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "IMF API returned status {} for {}",
            response.status(),
            indicator_code
        )));
    }

    let imf_data: ImfResponse = response.json().await.map_err(|e| {
        AppError::Parse(format!(
            "Failed to parse IMF response for {}: {}",
            indicator_code, e
        ))
    })?;

    let now = Utc::now().to_rfc3339();
    let mut inserted: usize = 0;

    // Build reverse lookup: IMF 3-letter -> our 2-letter
    let code_map: HashMap<&str, &str> = IMF_COUNTRIES
        .iter()
        .map(|(imf, ours)| (*imf, *ours))
        .collect();

    // Navigate: values -> indicator_code -> country -> year -> value
    if let Some(indicator_data) = imf_data.values.get(indicator_code) {
        for (imf_country, year_values) in indicator_data {
            let our_code = match code_map.get(imf_country.as_str()) {
                Some(c) => c,
                None => continue, // Skip countries not in our list
            };

            for (year, value_opt) in year_values {
                let value = match value_opt {
                    Some(v) => *v,
                    None => continue, // Skip null forecast values
                };

                let indicator_name = format!("IMF_{}_{}", indicator_code, our_code);

                // Dedup check
                let exists: bool = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM macro_data WHERE indicator = ?1 AND period = ?2",
                )
                .bind(&indicator_name)
                .bind(year)
                .fetch_one(pool)
                .await
                .map(|count| count > 0)
                .map_err(|e| {
                    AppError::Database(format!("Check existing macro_data failed: {}", e))
                })?;

                if exists {
                    // Update existing row — IMF revises forecasts semi-annually
                    sqlx::query(
                        r#"UPDATE macro_data SET value = ?1, fetched_at = ?2
                           WHERE indicator = ?3 AND period = ?4"#,
                    )
                    .bind(value)
                    .bind(&now)
                    .bind(&indicator_name)
                    .bind(year)
                    .execute(pool)
                    .await
                    .map_err(|e| AppError::Database(format!("Update macro_data failed: {}", e)))?;
                    continue;
                }

                sqlx::query(
                    r#"INSERT INTO macro_data (indicator, value, period, source, fetched_at)
                       VALUES (?1, ?2, ?3, 'imf', ?4)"#,
                )
                .bind(&indicator_name)
                .bind(value)
                .bind(year)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("Insert macro_data failed: {}", e)))?;

                inserted += 1;
            }
        }
    } else {
        log::warn!("IMF [{}]: indicator not found in response", indicator_code);
    }

    log::info!(
        "IMF [{}] ({}): {} new observations",
        indicator_code,
        label,
        inserted
    );
    Ok(inserted)
}

/// Fetch all configured IMF WEO indicators for 15 countries.
///
/// Rate-limits to 1 request per second to respect IMF's implicit limits.
/// On individual indicator failure, logs warning and continues.
pub async fn fetch_all_data(pool: &SqlitePool) -> Result<usize, AppError> {
    let mut total = 0;

    for (i, (code, label)) in IMF_INDICATORS.iter().enumerate() {
        // Rate limit: 1 second delay between requests (skip before first)
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        match fetch_indicator(pool, code, label).await {
            Ok(count) => total += count,
            Err(e) => {
                log::warn!("IMF [{}] ({}) failed: {}", code, label, e);
                // Continue with next indicator -- do not abort
            }
        }
    }

    Ok(total)
}
