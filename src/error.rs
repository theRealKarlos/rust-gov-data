use thiserror::Error;

/// Application error type for all expected error cases.
/// This allows for clear error handling and reporting throughout the app.
#[derive(Debug, Error)]
pub enum AppError {
    /// HTTP request failed (CKAN or S3)
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// CSV writing failed
    #[error("CSV write failed: {0}")]
    Csv(#[from] csv::Error),
    /// IO error (file operations)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Serde JSON error (parsing CKAN responses)
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// Configuration validation error (invalid or missing config values)
    #[error("Configuration error: {0}")]
    Config(String),
    /// Any other error (string message)
    #[error("Other error: {0}")]
    Other(String),
}
