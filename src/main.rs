// AWS SDK and Lambda runtime imports for interacting with AWS services and Lambda events.
use lambda_runtime::{run, service_fn, Error, LambdaEvent}; // Lambda runtime and event types
use serde::{Deserialize, Serialize}; // For (de)serialising JSON and CSV
use std::sync::Arc; // For sharing HTTP client across tasks
use futures::stream::StreamExt; // For concurrent async processing
use tracing::{info, error}; // For structured logging

mod config;
mod error;
mod ckan;
mod csv_writer;
mod s3_upload;

use config::Config;
use error::AppError;
use ckan::{fetch_dataset_list, fetch_dataset_metadata, create_http_client};
use csv_writer::write_csv;
use s3_upload::upload_to_s3;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub id: String,
    pub title: String,
    pub description: String,
    pub license: String,
    pub organization: String,
    pub created: String,
    pub modified: String,
    pub format: String,
}

async fn process_datasets(config: &Config, test_mode: bool) -> Result<(), AppError> {
    info!("Starting process_datasets: test_mode = {}", test_mode);
    // Use the optimized HTTP client with better connection pooling
    let client = Arc::new(create_http_client()?);
    let dataset_ids = fetch_dataset_list(&client, config, test_mode).await?;
    info!("Fetched {} dataset ids", dataset_ids.len());
    let concurrency_limit = config.concurrency_limit;
    info!("Starting concurrent metadata fetch for all datasets...");
    let metadata_results = futures::stream::iter(dataset_ids)
        .map(|id| {
            let client = Arc::clone(&client);
            let config = config.clone();
            async move {
                info!("Fetching metadata for dataset: {}", id);
                let result = fetch_dataset_metadata(client, &config, id.clone()).await;
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
    let mut dataset_metadata: Vec<(DatasetMetadata, Vec<String>)> = Vec::new();
    for res in metadata_results.into_iter() {
        if let Ok(Some((meta, urls))) = res {
            dataset_metadata.push((meta, urls));
        }
    }
    info!("Writing {} datasets to CSV...", dataset_metadata.len());
    write_csv(config, &dataset_metadata)?;
    info!("CSV file written: {}", config.csv_file);
    upload_to_s3(config, &config.csv_file).await?;
    info!("CSV file uploaded to S3 successfully.");
    Ok(())
}

// Lambda handler function. This is the entry point for AWS Lambda.
// It can also be called locally for testing.
async fn function_handler(event: LambdaEvent<serde_json::Value>) -> Result<serde_json::Value, Error> {
    let test_mode = event.payload.get("test_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| std::env::var("TEST_MODE").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false));
    info!("Lambda handler invoked. test_mode = {}", test_mode);
    let config = Config::new();
    process_datasets(&config, test_mode).await.map_err(|e| Error::from(e.to_string()))?;
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
        .with_ansi(false) // Disable colour codes for cleaner logs in CloudWatch
        .init();
    // Validate configuration before starting
    let config = Config::new();
    if let Err(e) = config.validate() {
        error!("Configuration error: {}", e);
        std::process::exit(1);
    }
    // Run the Lambda runtime with our handler.
    if let Err(e) = run(service_fn(function_handler)).await {
        error!("Lambda runtime error: {}", e);
    }
}

// Unit tests are in a separate file (src/tests.rs) for clarity and maintainability.
#[cfg(test)]
mod tests;