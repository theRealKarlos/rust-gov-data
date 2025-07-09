use crate::config::Config;
use crate::error::AppError;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

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

pub async fn fetch_dataset_list(client: &Client, config: &Config, test_mode: bool) -> Result<Vec<String>, AppError> {
    let response = client.get(&config.dataset_list_url())
        .timeout(std::time::Duration::from_secs(10))
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
    use regex::Regex;
    let url = format!("{}{}", config.dataset_metadata_url(), dataset_id);
    let response = client.get(&url)
        .timeout(std::time::Duration::from_secs(10))
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
        let re = Regex::new(r"<[^>]+>").expect("Regex should compile");
        let clean_description = re.replace_all(&dataset.notes, "").to_string();
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