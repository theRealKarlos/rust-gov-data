use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use tokio::time::{Duration};
use futures::stream::{self, StreamExt};
use anyhow::{Context, Result};
// Import tracing for structured, leveled logging (better than println! for production/Lambda)
use tracing::{info, error};

// Generic helper for environment variables with defaults.
fn get_env_or_default(var: &str, default: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| default.to_string())
}

// Use generic helper for all config getters.
fn get_ckan_api_base_url() -> String {
    get_env_or_default("CKAN_API_BASE_URL", "https://ckan.publishing.service.gov.uk/api/action")
}
fn get_dataset_list_url() -> String {
    format!("{}/package_list", get_ckan_api_base_url())
}
fn get_dataset_metadata_url() -> String {
    format!("{}/package_show?id=", get_ckan_api_base_url())
}
fn get_bucket_name() -> String {
    get_env_or_default("BUCKET_NAME", "gov-data-lucky4some.com")
}
fn get_csv_file() -> String {
    get_env_or_default("CSV_FILE", "DataGovUK_Datasets.csv")
}
fn get_concurrency_limit() -> usize {
    get_env_or_default("CONCURRENCY_LIMIT", "10").parse().unwrap_or(10)
}

// Strongly-typed struct for CKAN package_list response.
// This improves type safety and makes the code more robust to API changes.
#[derive(Debug, Deserialize)]
struct PackageListResponse {
    result: Vec<String>,
}

// Strongly-typed struct for CKAN package_show response.
// Using Option allows us to handle missing or null results gracefully.
#[derive(Debug, Deserialize)]
struct PackageShowResponse {
    result: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatasetMetadata {
    id: String,
    title: String,
    description: String,
    license: String,
    organization: String,
    created: String,
    modified: String,
    format: String,
    download_urls: String,
}

// Helper to extract resource formats as a comma-separated string and URLs as a JSON array string from the CKAN API response.
// CKAN 'resources' is an array of objects, each with fields like 'url' and 'format'.
// This function collects all 'format' fields and all 'url' fields from the resource objects.
fn extract_resource_formats_and_urls(result: &serde_json::Value) -> (String, String) {
    let resources = result.get("resources").and_then(|v| v.as_array());
    // Collect all 'format' fields as a comma-separated string.
    let formats = resources
        .map(|arr| {
            arr.iter()
                .filter_map(|res| res.get("format").and_then(|f| f.as_str()))
                .collect::<Vec<&str>>()
                .join(", ")
        })
        .unwrap_or_default();
    // Collect all 'url' fields as a Vec<&str>.
    let urls = resources
        .map(|arr| {
            arr.iter()
                .filter_map(|res| res.get("url").and_then(|u| u.as_str()))
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    // Serialize the URLs as a JSON array string for storage in the CSV.
    let urls_json = to_string(&urls).unwrap_or("[]".to_string());
    (formats, urls_json)
}

// Centralized test mode truncation logic.
fn maybe_truncate_for_test_mode<T: Clone>(items: Vec<T>, test_mode: bool, max: usize) -> Vec<T> {
    if test_mode {
        items.into_iter().take(max).collect()
    } else {
        items
    }
}

// Fetch the list of dataset IDs from the CKAN API.
// Uses a strongly-typed struct for safety.
async fn fetch_dataset_list(client: &Client, test_mode: bool) -> Result<Vec<String>, Error> {
    info!("Fetching dataset list...");
    let response = client.get(&get_dataset_list_url())
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    info!("Response received: {:?}", response.status());
    let package_list: PackageListResponse = response.json().await?;
    // Use centralized truncation logic for test mode
    Ok(maybe_truncate_for_test_mode(package_list.result, test_mode, 20))
}

// Fetch detailed metadata for a single dataset from the CKAN API.
// Uses a strongly-typed struct for the response and cleans up HTML in descriptions.
async fn fetch_dataset_metadata(client: Arc<Client>, dataset_id: String) -> Result<Option<DatasetMetadata>, Error> {
    use regex::Regex;
    let url = format!("{}{}", get_dataset_metadata_url(), dataset_id);
    let response = client.get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    info!("Metadata request for {} status: {:?}", dataset_id, response.status());
    if response.status().is_success() {
        let metadata: PackageShowResponse = response.json().await?;
        let result = match &metadata.result {
            Some(val) => val,
            None => {
                error!("No result for dataset {}", dataset_id);
                return Ok(None);
            }
        };
        let (formats, urls_json) = extract_resource_formats_and_urls(result);
        let notes = result["notes"].as_str().unwrap_or("");
        let re = Regex::new(r"<[^>]+>").expect("Regex should compile");
        let clean_description = re.replace_all(notes, "").to_string();
        return Ok(Some(DatasetMetadata {
            id: result["id"].as_str().unwrap_or_default().to_string(),
            title: result["title"].as_str().unwrap_or_default().to_string(),
            description: clean_description,
            license: result["license_title"].as_str().unwrap_or_default().to_string(),
            organization: result["organization"]["title"].as_str().unwrap_or_default().to_string(),
            created: result["metadata_created"].as_str().unwrap_or_default().to_string(),
            modified: result["metadata_modified"].as_str().unwrap_or_default().to_string(),
            format: formats,
            download_urls: urls_json,
        }));
    }
    error!("Failed to fetch metadata for {}", dataset_id);
    Ok(None)
}

// Main processing function: fetches dataset IDs, fetches metadata concurrently, writes CSV, uploads to S3, and handles test mode.
async fn process_datasets(test_mode: bool) -> Result<(), Error> {
    info!("Starting process_datasets: test_mode = {}", test_mode);
    let client = Arc::new(Client::builder()
        .pool_max_idle_per_host(5)
        .timeout(Duration::from_secs(10))
        .build()?);
    let dataset_ids = fetch_dataset_list(&client, test_mode).await?;
    info!("Fetched {} dataset ids", dataset_ids.len());
    // No need to truncate again here; handled in fetch_dataset_list
    let concurrency_limit = get_concurrency_limit();
    info!("Starting concurrent metadata fetch for all datasets...");
    let metadata_results = stream::iter(dataset_ids)
        .map(|id| {
            let client = Arc::clone(&client);
            async move {
                info!("Fetching metadata for dataset: {}", id);
                let result = fetch_dataset_metadata(client, id.clone()).await;
                match &result {
                    Ok(Some(_)) => info!("Finished fetching metadata for dataset: {}", id),
                    Ok(None) => error!("No metadata found for dataset: {}", id),
                    Err(e) => error!("Error fetching metadata for dataset {}: {}", id, e),
                }
                result
            }
        })
        .buffered(concurrency_limit)
        .collect::<Vec<_>>()
        .await;
    info!("Finished concurrent metadata fetch for all datasets.");
    let dataset_metadata: Vec<DatasetMetadata> = metadata_results.into_iter()
        .filter_map(|res| match res {
            Ok(Some(data)) => Some(data),
            _ => None,
        })
        .collect();
    if !dataset_metadata.is_empty() {
        let csv_file = get_csv_file();
        let file = File::create(&csv_file).context("Failed to create CSV file")?;
        let mut wtr = csv::Writer::from_writer(file);
        for dataset in dataset_metadata.iter() {
            wtr.serialize(dataset).context("Failed to serialize dataset to CSV")?;
        }
        wtr.flush().context("Failed to flush CSV writer")?;
        info!("CSV file written: {}", csv_file);
        upload_to_s3(&csv_file).await?;
        // Removed: local copy in test mode
    }
    info!("process_datasets finished successfully.");
    Ok(())
}

// Upload the CSV file to S3. Uses error context for robust error reporting.
async fn upload_to_s3(csv_file: &str) -> Result<(), Error> {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&config);
    let mut file = File::open(csv_file).context("Failed to open CSV file for S3 upload")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Failed to read CSV file for S3 upload")?;
    s3_client.put_object()
        .bucket(get_bucket_name())
        .key("DataGovUK_Datasets.csv")
        .body(ByteStream::from(buffer))
        .send()
        .await?;
    info!("CSV successfully uploaded to S3.");
    Ok(())
}

async fn function_handler(event: LambdaEvent<serde_json::Value>) -> Result<serde_json::Value, Error> {
    println!("Lambda event payload: {:?}", event.payload); // Debug print
    let test_mode = event.payload.get("test_mode").and_then(|v| v.as_bool())
        .or_else(|| {
            // Try to parse from a stringified body if present
            event.payload.get("body")
                .and_then(|v| v.as_str())
                .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
                .and_then(|v| v.get("test_mode").and_then(|v| v.as_bool()))
        })
        .unwrap_or(false);
    println!("function_handler: test_mode = {}", test_mode); // Debug print

    match process_datasets(test_mode).await {
        Ok(_) => Ok(serde_json::json!({ "message": "Dataset metadata stored in S3", "test_mode": test_mode })),
        Err(e) => {
            error!("process_datasets failed: {:?}", e);
            // Return a JSON error message but do not propagate the error, so Lambda exits cleanly
            Ok(serde_json::json!({
                "error": format!("process_datasets failed: {}", e),
                "test_mode": test_mode
            }))
        }
    }
}

// Initialize tracing for structured logging in main().
// This ensures logs are visible in both local and Lambda environments.
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run(service_fn(function_handler)).await {
        error!("Lambda runtime exited with error: {:?}", e);
    }
}

#[cfg(test)]
mod tests;