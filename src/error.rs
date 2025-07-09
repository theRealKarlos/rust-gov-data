use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("CSV write failed: {0}")]
    Csv(#[from] csv::Error),
    #[error("S3 upload failed: {0}")]
    S3(#[from] aws_sdk_s3::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
} 