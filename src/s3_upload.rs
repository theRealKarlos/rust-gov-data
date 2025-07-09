use crate::config::Config;
use crate::error::AppError;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use std::fs::File;
use std::io::Read;

pub async fn upload_to_s3(config: &Config, csv_file: &str) -> Result<(), AppError> {
    let region_provider = RegionProviderChain::default_provider().or_else("eu-west-2");
    let aws_config = aws_config::from_env().region(region_provider).load().await;
    let client = S3Client::new(&aws_config);
    let bucket = &config.bucket_name;
    let key = csv_file.split('/').last().unwrap_or(csv_file);
    let mut file = File::open(csv_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buffer))
        .send()
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(())
} 