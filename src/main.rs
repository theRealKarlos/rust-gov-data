// AWS SDK and Lambda runtime imports for interacting with AWS services and Lambda events.
use aws_config::meta::region::RegionProviderChain; // For region configuration
use aws_sdk_s3::primitives::ByteStream; // For S3 file upload
use aws_sdk_s3::Client as S3Client; // S3 client
use lambda_runtime::{run, service_fn, Error, LambdaEvent}; // Lambda runtime and event types
use reqwest::Client; // HTTP client for CKAN API
use serde::{Deserialize, Serialize}; // For (de)serialising JSON and CSV
use std::fs::File; // For file operations
use std::io::Read; // For reading file contents
use std::sync::Arc; // For sharing HTTP client across tasks
use tokio::time::{Duration}; // For timeouts
use futures::stream::{self, StreamExt}; // For concurrent async processing
use anyhow::{Context, Result}; // For error context and handling
use tracing::{info, error}; // For structured logging

// Helper to get an environment variable or use a default value if not set.
fn get_env_or_default(var: &str, default: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| default.to_string())
}

// Helper functions to get configuration values from environment variables or defaults.
// These make the code flexible for different environments and easy to configure.
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
// Returns the CSV file path. In AWS Lambda, always use /tmp/ (the only writable directory).
fn get_csv_file() -> String {
    let filename = get_env_or_default("CSV_FILE", "DataGovUK_Datasets.csv");
    // If running in Lambda, always use /tmp/
    if std::env::var("LAMBDA_TASK_ROOT").is_ok() {
        format!("/tmp/{}", filename)
    } else {
        filename
    }
}
fn get_concurrency_limit() -> usize {
    get_env_or_default("CONCURRENCY_LIMIT", "10").parse().unwrap_or(10)
}

// Struct for the CKAN package_list API response.
// Contains a list of dataset IDs.
#[derive(Debug, Deserialize)]
struct PackageListResponse {
    result: Vec<String>,
}

// Struct for the CKAN package_show API response.
// Contains detailed metadata for a dataset.
#[derive(Debug, Deserialize)]
struct PackageShowResponse {
    result: Option<serde_json::Value>,
}

// Struct for storing dataset metadata in CSV and S3.
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
    // download_urls: String, // Removed, handled separately
}

// Helper to extract resource formats as a comma-separated string and URLs as a JSON array string from the CKAN API response.
// CKAN 'resources' is an array of objects, each with fields like 'url' and 'format'.
// This function collects all 'format' fields and all 'url' fields from the resource objects.
fn extract_resource_formats_and_urls(result: &serde_json::Value) -> (String, Vec<String>) {
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
    // Collect all 'url' fields as a Vec<String>.
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

// Helper to truncate a vector for test mode, limiting the number of items processed.
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
async fn fetch_dataset_metadata(client: Arc<Client>, dataset_id: String) -> Result<Option<(DatasetMetadata, Vec<String>)>, Error> {
    use regex::Regex; // For cleaning HTML tags from descriptions
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
        let (formats, urls_vec) = extract_resource_formats_and_urls(result);
        let notes = result["notes"].as_str().unwrap_or("");
        // Remove HTML tags from the description for cleaner output.
        let re = Regex::new(r"<[^>]+>").expect("Regex should compile");
        let clean_description = re.replace_all(notes, "").to_string();
        return Ok(Some((DatasetMetadata {
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
    error!("Failed to fetch metadata for {}", dataset_id);
    Ok(None)
}

// Main processing function: fetches dataset IDs, fetches metadata concurrently, writes CSV, uploads to S3, and handles test mode.
// This is the main workflow for the Lambda function.
async fn process_datasets(test_mode: bool) -> Result<(), Error> {
    info!("Starting process_datasets: test_mode = {}", test_mode);
    // Create a shared HTTP client for efficient connection reuse.
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
    // Separate metadata and urls, and find max number of URLs
    let mut dataset_metadata: Vec<(DatasetMetadata, Vec<String>)> = Vec::new();
    let mut max_urls = 0;
    for res in metadata_results.into_iter() {
        if let Ok(Some((meta, urls))) = res {
            max_urls = max_urls.max(urls.len());
            dataset_metadata.push((meta, urls));
        }
    }
    info!("Writing {} datasets to CSV...", dataset_metadata.len());
    let csv_file = get_csv_file();
    // Write the dataset metadata to a CSV file. Use error context for easier debugging.
    {
        let file = File::create(&csv_file).context("Failed to create CSV file")?;
        let mut wtr = csv::Writer::from_writer(file);
        // Write header
        let mut header = vec![
            "id".to_string(), "title".to_string(), "description".to_string(), "license".to_string(),
            "organization".to_string(), "created".to_string(), "modified".to_string(), "format".to_string()
        ];
        for i in 1..=max_urls {
            header.push(format!("download_url_{}", i));
        }
        wtr.write_record(&header)?;
        // Write rows
        for (meta, urls) in &dataset_metadata {
            let mut row = vec![
                meta.id.clone(),
                meta.title.clone(),
                meta.description.clone(),
                meta.license.clone(),
                meta.organization.clone(),
                meta.created.clone(),
                meta.modified.clone(),
                meta.format.clone(),
            ];
            // Add each url, pad with empty if fewer than max_urls
            for i in 0..max_urls {
                if i < urls.len() {
                    row.push(urls[i].clone());
                } else {
                    row.push(String::new());
                }
            }
            wtr.write_record(&row)?;
        }
        wtr.flush().context("Failed to flush CSV writer")?;
    }
    info!("CSV file written: {}", csv_file);
    // Upload the CSV file to S3. Uses error context for robust error reporting.
    upload_to_s3(&csv_file).await?;
    info!("CSV file uploaded to S3 successfully.");
    Ok(())
}

// Uploads the given CSV file to the configured S3 bucket.
// Reads the file into memory and uploads it as a ByteStream.
async fn upload_to_s3(csv_file: &str) -> Result<(), Error> {
    info!("Uploading {} to S3 bucket...", csv_file);
    // Load AWS region from environment or default provider chain.
    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-2");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = S3Client::new(&config);
    let bucket = get_bucket_name();
    let key = csv_file.split('/').last().unwrap_or(csv_file);
    // Read the file into a buffer for upload.
    let mut file = File::open(csv_file).context("Failed to open CSV file for S3 upload")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Failed to read CSV file for S3 upload")?;
    // Upload the file to S3.
    client.put_object()
        .bucket(&bucket)
        .key(key)
        .body(ByteStream::from(buffer))
        .send()
        .await?;
    info!("File uploaded to S3: bucket={}, key={}", bucket, key);
    Ok(())
}

// Lambda handler function. This is the entry point for AWS Lambda.
// It can also be called locally for testing.
async fn function_handler(event: LambdaEvent<serde_json::Value>) -> Result<serde_json::Value, Error> {
    // Check for test mode in the event payload or environment variable.
    let test_mode = event.payload.get("test_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| std::env::var("TEST_MODE").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false));
    info!("Lambda handler invoked. test_mode = {}", test_mode);
    // Run the main processing logic.
    process_datasets(test_mode).await?;
    // Return a success message as JSON.
    Ok(serde_json::json!({ "status": "success" }))
}

// Main function for the binary. Sets up logging and runs the Lambda runtime.
#[tokio::main]
async fn main() {
    // Initialise tracing subscriber for logging. This works for both local and Lambda environments.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    // Run the Lambda runtime with our handler.
    if let Err(e) = run(service_fn(function_handler)).await {
        error!("Lambda runtime error: {}", e);
    }
}

// Unit tests are in a separate file (src/tests.rs) for clarity and maintainability.
#[cfg(test)]
mod tests;