// Centralised configuration struct for all application settings.
// This makes the code more maintainable and easier to test.

/// Configuration for the application, loaded from environment variables or defaults.
#[derive(Debug, Clone)]
pub struct Config {
    /// The base URL for the CKAN API.
    pub ckan_api_base_url: String,
    /// The S3 bucket name for output.
    pub bucket_name: String,
    /// The output CSV file name or path.
    pub csv_file: String,
    /// The concurrency limit for async processing.
    pub concurrency_limit: usize,
}

impl Config {
    /// Create a new Config by reading environment variables or using defaults.
    pub fn new() -> Self {
        Self {
            ckan_api_base_url: Self::get_env_or_default(
                "CKAN_API_BASE_URL",
                "https://ckan.publishing.service.gov.uk/api/action",
            ),
            bucket_name: Self::get_env_or_default("BUCKET_NAME", "gov-data-lucky4some.com"),
            csv_file: Self::get_csv_file(),
            concurrency_limit: Self::get_env_or_default("CONCURRENCY_LIMIT", "10")
                .parse()
                .unwrap_or(10),
        }
    }

    /// Validate the configuration, returning an error if any required value is missing or invalid.
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.ckan_api_base_url.trim().is_empty() {
            return Err(crate::error::AppError::Config(
                "CKAN API base URL must not be empty".to_string(),
            ));
        }
        if self.bucket_name.trim().is_empty() {
            return Err(crate::error::AppError::Config(
                "S3 bucket name must not be empty".to_string(),
            ));
        }
        if self.csv_file.trim().is_empty() {
            return Err(crate::error::AppError::Config(
                "CSV file name must not be empty".to_string(),
            ));
        }
        if self.concurrency_limit == 0 {
            return Err(crate::error::AppError::Config(
                "Concurrency limit must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }

    /// Helper to get an environment variable or use a default value if not set.
    fn get_env_or_default(var: &str, default: &str) -> String {
        std::env::var(var).unwrap_or_else(|_| default.to_string())
    }

    /// Returns the CSV file path. In AWS Lambda, always use /tmp/ (the only writable directory).
    fn get_csv_file() -> String {
        let filename = Self::get_env_or_default("CSV_FILE", "DataGovUK_Datasets.csv");
        // If running in Lambda, always use /tmp/
        if std::env::var("LAMBDA_TASK_ROOT").is_ok() {
            format!("/tmp/{filename}")
        } else {
            filename
        }
    }

    /// Get the CKAN dataset list URL.
    pub fn dataset_list_url(&self) -> String {
        format!("{}/package_list", self.ckan_api_base_url)
    }

    /// Get the CKAN dataset metadata URL prefix.
    pub fn dataset_metadata_url(&self) -> String {
        format!("{}/package_show?id=", self.ckan_api_base_url)
    }
}
