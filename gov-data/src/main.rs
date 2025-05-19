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

const DATASET_LIST_URL: &str = "https://ckan.publishing.service.gov.uk/api/action/package_list";
const DATASET_METADATA_URL: &str = "https://ckan.publishing.service.gov.uk/api/action/package_show?id=";
const BUCKET_NAME: &str = "gov-data-lucky4some.com";
const CSV_FILE: &str = "DataGovUK_Datasets.csv"; // Lambda allows writing to /tmp

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

async fn fetch_dataset_list(client: &Client, test_mode: bool) -> Result<Vec<String>, Error> {
    println!("Fetching dataset list...");
    
    let response = client.get(DATASET_LIST_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    println!("Response received: {:?}", response.status());

    let dataset_ids: Vec<String> = response.json::<serde_json::Value>().await?["result"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|id| id.as_str().map(String::from))
        .collect();

    let dataset_ids = if test_mode {
        dataset_ids.iter().take(20).cloned().collect::<Vec<String>>() // Prevents ownership issues
    } else {
        dataset_ids
    };

    Ok(dataset_ids)
}

async fn fetch_dataset_metadata(client: Arc<Client>, dataset_id: String) -> Result<Option<DatasetMetadata>, Error> {
    use regex::Regex;
    let url = format!("{}{}", DATASET_METADATA_URL, dataset_id);
    let response = client.get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    println!("Metadata request for {} status: {:?}", dataset_id, response.status());

    if response.status().is_success() {
        let metadata = response.json::<serde_json::Value>().await?;
        let result = &metadata["result"];
        if result.is_null() {
            println!("No result for dataset {}", dataset_id);
            return Ok(None);
        }
        let resources = result["resources"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|res| res["url"].as_str().map(String::from))
            .collect::<Vec<String>>();

        let urls_json = to_string(&resources).unwrap_or_else(|_| "[]".to_string());

        // Use regex to strip HTML tags from description
        let notes = result["notes"].as_str().unwrap_or("");
        let re = Regex::new(r"<[^>]+>").unwrap();
        let clean_description = re.replace_all(notes, "").to_string();

        return Ok(Some(DatasetMetadata {
            id: result["id"].as_str().unwrap_or_default().to_string(),
            title: result["title"].as_str().unwrap_or_default().to_string(),
            description: clean_description,
            license: result["license_title"].as_str().unwrap_or_default().to_string(),
            organization: result["organization"]["title"].as_str().unwrap_or_default().to_string(),
            created: result["metadata_created"].as_str().unwrap_or_default().to_string(),
            modified: result["metadata_modified"].as_str().unwrap_or_default().to_string(),
            format: result["resources"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|res| res["format"].as_str())
                .collect::<Vec<&str>>()
                .join(", "),
            download_urls: urls_json,
        }));
    }

    println!("Failed to fetch metadata for {}", dataset_id);
    Ok(None)
}

async fn process_datasets(test_mode: bool) -> Result<(), Error> {
    println!("process_datasets: test_mode = {}", test_mode);
    let client = Arc::new(Client::builder()
        .pool_max_idle_per_host(5)
        .timeout(Duration::from_secs(10))
        .build()?);

    let mut dataset_ids = fetch_dataset_list(&client, test_mode).await?;
    println!("Fetched {} dataset ids", dataset_ids.len());
    if test_mode && dataset_ids.len() > 20 {
        dataset_ids.truncate(1);
        println!("Test mode: truncated to 20 dataset id");
    }

    let concurrency_limit = 10; // Limit concurrent requests
    let metadata_results = stream::iter(dataset_ids)
        .map(|id| {
            let client = Arc::clone(&client);
            async move { fetch_dataset_metadata(client, id).await }
        })
        .buffer_unordered(concurrency_limit)
        .collect::<Vec<_>>()
        .await;

    let dataset_metadata: Vec<DatasetMetadata> = metadata_results.into_iter()
        .filter_map(|res| match res {
            Ok(Some(data)) => Some(data),
            _ => None,
        })
        .collect();

    if !dataset_metadata.is_empty() {
        let file = File::create(CSV_FILE)?;
        let mut wtr = csv::Writer::from_writer(file);
        for dataset in dataset_metadata.iter() {
            wtr.serialize(dataset)?;
        }
        wtr.flush()?;

        upload_to_s3().await?;
    }

    Ok(())
}

async fn upload_to_s3() -> Result<(), Error> {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&config);

    let mut file = File::open(CSV_FILE)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    s3_client.put_object()
        .bucket(BUCKET_NAME)
        .key("DataGovUK_Datasets.csv")
        .body(ByteStream::from(buffer))
        .send()
        .await?;

    println!("CSV successfully uploaded to S3.");
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
        Err(e) => Err(e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}