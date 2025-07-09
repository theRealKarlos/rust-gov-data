// Centralised configuration struct for all application settings.
// This makes the code more maintainable and easier to test.
#[derive(Debug, Clone)]
pub struct Config {
    pub ckan_api_base_url: String,
    pub bucket_name: String,
    pub csv_file: String,
    pub concurrency_limit: usize,
}

impl Config {
    pub fn new() -> Self {
        Self {
            ckan_api_base_url: Self::get_env_or_default(
                "CKAN_API_BASE_URL", 
                "https://ckan.publishing.service.gov.uk/api/action"
            ),
            bucket_name: Self::get_env_or_default("BUCKET_NAME", "gov-data-lucky4some.com"),
            csv_file: Self::get_csv_file(),
            concurrency_limit: Self::get_env_or_default("CONCURRENCY_LIMIT", "10")
                .parse()
                .unwrap_or(10),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ckan_api_base_url.trim().is_empty() {
            return Err("CKAN API base URL must not be empty".to_string());
        }
        if self.bucket_name.trim().is_empty() {
            return Err("S3 bucket name must not be empty".to_string());
        }
        if self.csv_file.trim().is_empty() {
            return Err("CSV file name must not be empty".to_string());
        }
        if self.concurrency_limit == 0 {
            return Err("Concurrency limit must be greater than zero".to_string());
        }
        Ok(())
    }

    // Helper to get an environment variable or use a default value if not set.
    fn get_env_or_default(var: &str, default: &str) -> String {
        std::env::var(var).unwrap_or_else(|_| default.to_string())
    }

    // Returns the CSV file path. In AWS Lambda, always use /tmp/ (the only writable directory).
    fn get_csv_file() -> String {
        let filename = Self::get_env_or_default("CSV_FILE", "DataGovUK_Datasets.csv");
        // If running in Lambda, always use /tmp/
        if std::env::var("LAMBDA_TASK_ROOT").is_ok() {
            format!("/tmp/{}", filename)
        } else {
            filename
        }
    }

    pub fn dataset_list_url(&self) -> String {
        format!("{}/package_list", self.ckan_api_base_url)
    }

    pub fn dataset_metadata_url(&self) -> String {
        format!("{}/package_show?id=", self.ckan_api_base_url)
    }
} 