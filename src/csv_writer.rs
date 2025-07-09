use crate::config::Config;
use crate::error::AppError;
use crate::DatasetMetadata;
use std::fs::File;

pub fn write_csv(
    config: &Config,
    dataset_metadata: &[(DatasetMetadata, Vec<String>)]
) -> Result<(), AppError> {
    let max_urls = dataset_metadata.iter().map(|(_, urls)| urls.len()).max().unwrap_or(0);
    let file = File::create(&config.csv_file)?;
    let mut wtr = csv::Writer::from_writer(file);
    let mut header = vec![
        "id".to_string(), "title".to_string(), "description".to_string(), "license".to_string(),
        "organization".to_string(), "created".to_string(), "modified".to_string(), "format".to_string()
    ];
    for i in 1..=max_urls {
        header.push(format!("download_url_{}", i));
    }
    wtr.write_record(&header)?;
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