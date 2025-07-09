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
    pub result: Option<serde_json::Value>,
}

pub fn extract_resource_formats_and_urls(result: &serde_json::Value) -> (String, Vec<String>) {
    let resources = result.get("resources").and_then(|v| v.as_array());
    let formats = resources
        .map(|arr| {
            arr.iter()
                .filter_map(|res| res.get("format").and_then(|f| f.as_str()))
                .collect::<Vec<&str>>()
                .join(", ")
        })
        .unwrap_or_default();
    let urls = resources
        .map(|arr| {
            arr.iter()
                .filter_map(|res| res.get("url").and_then(|u| u.as_str()))
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
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
        let result = match &metadata.result {
            Some(val) => val,
            None => {
                return Ok(None);
            }
        };
        let (formats, urls_vec) = extract_resource_formats_and_urls(result);
        let notes = result["notes"].as_str().unwrap_or("");
        let re = Regex::new(r"<[^>]+>").expect("Regex should compile");
        let clean_description = re.replace_all(notes, "").to_string();
        return Ok(Some((crate::DatasetMetadata {
            id: result["id"].as_str().unwrap_or_default().to_string(),
            title: result["title"].as_str().unwrap_or_default().to_string(),
            description: clean_description,
            license: result["license_title"].as_str().unwrap_or_default().to_string(),
            organization: result["organization"]["title"].as_str().unwrap_or_default().to_string(),
            created: result["metadata_created"].as_str().unwrap_or_default().to_string(),
            modified: result["metadata_modified"].as_str().unwrap_or_default().to_string(),
            format: formats,
        }, urls_vec)));
    }
    Ok(None)
} 