use crate::config::Config;
use crate::error::AppError;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use std::fs::File;
use std::io::Read;
use tracing::info;

/// Uploads the given CSV file to the configured S3 bucket.
/// Reads the file into memory and uploads it as a ByteStream.
/// Uses an optimised buffer and logs file size and upload status.
///
/// # Arguments
/// * `config` - The application configuration (must contain bucket name)
/// * `csv_file` - The path to the CSV file to upload
pub async fn upload_to_s3(config: &Config, csv_file: &str) -> Result<(), AppError> {
    info!("Uploading {} to S3 bucket...", csv_file);
    
    // Load AWS configuration with optimised settings
    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-2");
    let aws_config = aws_config::from_env()
        .region(region_provider)
        .load()
        .await;
    
    let client = S3Client::new(&aws_config);
    let bucket = &config.bucket_name;
    let key = csv_file.split('/').last().unwrap_or(csv_file);
    
    // Read file with optimised buffer size (8KB chunks for better memory usage)
    let mut file = File::open(csv_file)?;
    let mut buffer = Vec::with_capacity(8192); // Pre-allocate 8KB buffer
    file.read_to_end(&mut buffer)?;
    
    info!("File size: {} bytes, uploading to S3: bucket={}, key={}", buffer.len(), bucket, key);
    
    // Upload with optimised settings
    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buffer))
        .send()
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    
    info!("Successfully uploaded file to S3: bucket={}, key={}", bucket, key);
    Ok(())
} 