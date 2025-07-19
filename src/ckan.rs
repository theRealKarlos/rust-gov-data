use crate::config::Config;
use crate::error::AppError;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

// Compile regex once and reuse it for HTML tag removal for performance.
static HTML_TAG_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("HTML tag regex should compile"));

/// Response from the CKAN package_list API.
#[derive(Debug, Deserialize)]
pub struct PackageListResponse {
    /// List of dataset IDs returned by the CKAN API.
    pub result: Vec<String>,
}

/// Response from the CKAN package_show API.
#[derive(Debug, Deserialize)]
pub struct PackageShowResponse {
    /// The dataset metadata, or None if not found.
    pub result: Option<CkanDataset>,
}

/// Strongly-typed struct for CKAN dataset metadata.
#[derive(Debug, Deserialize)]
pub struct CkanDataset {
    /// Dataset ID
    pub id: String,
    /// Dataset title
    pub title: String,
    /// Dataset description (may contain HTML)
    pub notes: String,
    /// License title
    pub license_title: String,
    /// Organisation info
    pub organization: CkanOrganization,
    /// Creation timestamp
    pub metadata_created: String,
    /// Modification timestamp
    pub metadata_modified: String,
    /// List of resources (files, links, etc.)
    pub resources: Vec<CkanResource>,
}

/// Organisation info for a CKAN dataset.
#[derive(Debug, Deserialize)]
pub struct CkanOrganization {
    /// Organisation title
    pub title: String,
}

/// Resource (file, link, etc.) for a CKAN dataset.
#[derive(Debug, Deserialize)]
pub struct CkanResource {
    /// File format (e.g., CSV, JSON)
    pub format: Option<String>,
    /// Download URL
    pub url: Option<String>,
}

/// Extracts resource formats as a comma-separated string and URLs as a Vec<String> from a CKAN dataset.
/// This is used to flatten the resource info for CSV output.
pub fn extract_resource_formats_and_urls(dataset: &CkanDataset) -> (String, Vec<String>) {
    let formats = dataset
        .resources
        .iter()
        .filter_map(|res| res.format.as_deref())
        .collect::<Vec<&str>>()
        .join(", ");
    let urls = dataset
        .resources
        .iter()
        .filter_map(|res| res.url.clone())
        .collect::<Vec<String>>();
    (formats, urls)
}

/// Creates an optimised HTTP client with connection pooling and timeouts for efficient API access.
pub fn create_http_client(config: &Config) -> Result<Client, AppError> {
    Ok(Client::builder()
        .pool_max_idle_per_host(10) // Increased from 5 for better concurrency
        .pool_idle_timeout(std::time::Duration::from_secs(90)) // Keep connections alive longer
        .timeout(std::time::Duration::from_secs(config.http_timeout_secs)) // Configurable timeout
        .connect_timeout(std::time::Duration::from_secs(10)) // Add connection timeout
        .tcp_keepalive(Some(std::time::Duration::from_secs(60))) // Enable TCP keepalive
        .build()?)
}

/// Fetches the list of dataset IDs from the CKAN API.
/// Returns a truncated list if test_mode is enabled.
pub async fn fetch_dataset_list(
    client: &Client,
    config: &Config,
    test_mode: bool,
) -> Result<Vec<String>, AppError> {
    let response = client
        .get(&config.dataset_list_url())
        .timeout(std::time::Duration::from_secs(config.http_timeout_secs))
        .send()
        .await?;
    let package_list: PackageListResponse = response.json().await?;
    Ok(if test_mode {
        package_list
            .result
            .into_iter()
            .take(config.test_mode_dataset_limit)
            .collect()
    } else {
        package_list.result
    })
}

/// Fetches detailed metadata for a single dataset from the CKAN API.
/// Cleans up HTML in the description and returns the metadata and download URLs.
pub async fn fetch_dataset_metadata(
    client: Arc<Client>,
    config: &Config,
    dataset_id: String,
) -> Result<Option<(crate::DatasetMetadata, Vec<String>)>, AppError> {
    let url = format!("{}{}", config.dataset_metadata_url(), dataset_id);
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(config.http_timeout_secs))
        .send()
        .await?;
    if response.status().is_success() {
        let metadata: PackageShowResponse = response.json().await?;
        let dataset = match &metadata.result {
            Some(val) => val,
            None => {
                return Ok(None);
            }
        };
        let (formats, urls_vec) = extract_resource_formats_and_urls(dataset);
        // Use the pre-compiled regex for better performance
        let clean_description = HTML_TAG_REGEX.replace_all(&dataset.notes, "").to_string();
        return Ok(Some((
            crate::DatasetMetadata {
                id: dataset.id.clone(),
                title: dataset.title.clone(),
                description: clean_description,
                license: dataset.license_title.clone(),
                organization: dataset.organization.title.clone(),
                created: dataset.metadata_created.clone(),
                modified: dataset.metadata_modified.clone(),
                format: formats,
            },
            urls_vec,
        )));
    }
    Ok(None)
}
