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
    /// S3 upload failed
    #[error("S3 upload failed: {0}")]
    S3(#[from] aws_sdk_s3::Error),
    /// IO error (file operations)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Serde JSON error (parsing CKAN responses)
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// Any other error (string message)
    #[error("Other error: {0}")]
    Other(String),
} 