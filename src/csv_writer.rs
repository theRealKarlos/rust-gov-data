use crate::config::Config;
use crate::error::AppError;
use crate::DatasetMetadata;
use std::fs::File;

/// Writes the dataset metadata to a CSV file with one column per download URL.
/// The number of download_url columns is determined by the dataset with the most URLs.
/// This function ensures the CSV is easy to use in Excel or other tools.
pub fn write_csv(
    config: &Config,
    dataset_metadata: &[(DatasetMetadata, Vec<String>)],
) -> Result<(), AppError> {
    // Find the maximum number of download URLs in any dataset for column generation.
    let max_urls = dataset_metadata
        .iter()
        .map(|(_, urls)| urls.len())
        .max()
        .unwrap_or(0);
    let file = File::create(&config.csv_file)?;
    let mut wtr = csv::Writer::from_writer(file);
    // Write the CSV header, including download_url_1, download_url_2, ...
    let mut header = vec![
        "id".to_string(),
        "title".to_string(),
        "description".to_string(),
        "license".to_string(),
        "organization".to_string(),
        "created".to_string(),
        "modified".to_string(),
        "format".to_string(),
    ];
    for i in 1..=max_urls {
        header.push(format!("download_url_{}", i));
    }
    wtr.write_record(&header)?;
    // Write each row, padding with empty strings if there are fewer URLs than max_urls.
    for (meta, urls) in dataset_metadata {
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
        for i in 0..max_urls {
            if i < urls.len() {
                row.push(urls[i].clone());
            } else {
                row.push(String::new());
            }
        }
        wtr.write_record(&row)?;
    }
    wtr.flush()?;
    Ok(())
}
