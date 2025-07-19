use crate::config::Config;
use crate::error::AppError;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use aws_types::region::Region;
use tracing::info;

/// Uploads the given CSV file to the configured S3 bucket.
/// Streams the file directly from the filesystem for memory efficiency.
/// Logs file size and upload status.
///
/// # Arguments
/// * `config` - The application configuration (must contain bucket name)
/// * `csv_file` - The path to the CSV file to upload
pub async fn upload_to_s3(config: &Config, csv_file: &str) -> Result<(), AppError> {
    info!("Uploading {} to S3 bucket...", csv_file);

    // Load AWS configuration with optimised settings
    let region_provider =
        RegionProviderChain::default_provider().or_else(Region::new(config.aws_region.clone()));
    let aws_config = aws_config::from_env().region(region_provider).load().await;

    let client = S3Client::new(&aws_config);
    let bucket = &config.bucket_name;
    let key = csv_file.split('/').next_back().unwrap_or(csv_file);

    // Use ByteStream::from_path for memory-efficient streaming upload
    let bytestream = ByteStream::from_path(csv_file)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;

    info!("Uploading file to S3: bucket={}, key={}", bucket, key);

    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(bytestream)
        .send()
        .await
        .map_err(|e| AppError::Other(format!("S3 upload failed: {e}")))?;

    info!(
        "Successfully uploaded file to S3: bucket={}, key={}",
        bucket, key
    );
    Ok(())
}
