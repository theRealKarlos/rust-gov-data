use crate::config::Config;
use crate::error::AppError;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;

// Compile regex once and reuse it for HTML tag removal
static HTML_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[^>]+>").expect("HTML tag regex should compile")
});

#[derive(Debug, Deserialize)]
pub struct PackageListResponse {
    pub result: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PackageShowResponse {
    pub result: Option<CkanDataset>,
}

#[derive(Debug, Deserialize)]
pub struct CkanDataset {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub license_title: String,
    pub organization: CkanOrganization,
    pub metadata_created: String,
    pub metadata_modified: String,
    pub resources: Vec<CkanResource>,
}

#[derive(Debug, Deserialize)]
pub struct CkanOrganization {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct CkanResource {
    pub format: Option<String>,
    pub url: Option<String>,
}

pub fn extract_resource_formats_and_urls(dataset: &CkanDataset) -> (String, Vec<String>) {
    let formats = dataset.resources.iter()
        .filter_map(|res| res.format.as_deref())
        .collect::<Vec<&str>>()
        .join(", ");
    let urls = dataset.resources.iter()
        .filter_map(|res| res.url.clone())
        .collect::<Vec<String>>();
    (formats, urls)
}

// Create an optimized HTTP client with better connection pooling
pub fn create_http_client() -> Result<Client, AppError> {
    Ok(Client::builder()
        .pool_max_idle_per_host(10) // Increased from 5
        .pool_idle_timeout(std::time::Duration::from_secs(90)) // Keep connections alive longer
        .timeout(std::time::Duration::from_secs(15)) // Increased timeout
        .connect_timeout(std::time::Duration::from_secs(10)) // Add connection timeout
        .tcp_keepalive(Some(std::time::Duration::from_secs(60))) // Enable TCP keepalive
        .build()?)
}

pub async fn fetch_dataset_list(client: &Client, config: &Config, test_mode: bool) -> Result<Vec<String>, AppError> {
    let response = client.get(&config.dataset_list_url())
        .timeout(std::time::Duration::from_secs(15)) // Increased timeout
        .send()
        .await?;
    let package_list: PackageListResponse = response.json().await?;
    Ok(if test_mode {
        package_list.result.into_iter().take(20).collect()
    } else {
        package_list.result
    })
}

pub async fn fetch_dataset_metadata(client: Arc<Client>, config: &Config, dataset_id: String) -> Result<Option<(crate::DatasetMetadata, Vec<String>)>, AppError> {
    let url = format!("{}{}", config.dataset_metadata_url(), dataset_id);
    let response = client.get(&url)
        .timeout(std::time::Duration::from_secs(15)) // Increased timeout
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
        return Ok(Some((crate::DatasetMetadata {
            id: dataset.id.clone(),
            title: dataset.title.clone(),
            description: clean_description,
            license: dataset.license_title.clone(),
            organization: dataset.organization.title.clone(),
            created: dataset.metadata_created.clone(),
            modified: dataset.metadata_modified.clone(),
            format: formats,
        }, urls_vec)));
    }
    Ok(None)
} 