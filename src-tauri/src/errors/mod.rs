use std::fmt;

/// Unified application error type.
/// Prefixed error codes for debugging: [NET_ERR], [DB_ERR], [PARSE_ERR], [KEY_ERR].
#[derive(Debug)]
pub enum AppError {
    /// Network / HTTP errors (e.g., RSS fetch, FRED API call)
    Network(String),
    /// Database errors (SQLite read/write)
    Database(String),
    /// Parse errors (RSS XML, JSON, data format)
    Parse(String),
    /// Missing or invalid API key
    ApiKey(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Network(msg) => write!(f, "[NET_ERR] {}", msg),
            AppError::Database(msg) => write!(f, "[DB_ERR] {}", msg),
            AppError::Parse(msg) => write!(f, "[PARSE_ERR] {}", msg),
            AppError::ApiKey(msg) => write!(f, "[KEY_ERR] {}", msg),
        }
    }
}

impl From<AppError> for String {
    fn from(err: AppError) -> String {
        err.to_string()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Network(err.to_string())
    }
}
