// Unit tests for gov-data (moved from main.rs)
// These tests check the parsing of CKAN API responses into strongly-typed Rust structs.
// They help ensure that changes to the API or our code are caught early.

use crate::ckan::PackageListResponse;
use crate::ckan::PackageShowResponse;
use crate::config::Config;

#[test]
fn test_parse_package_list_response() {
    // Test that a valid package list response is parsed into the struct.
    let data = serde_json::json!({ "result": ["dataset1", "dataset2"] });
    let parsed: PackageListResponse = serde_json::from_value(data).unwrap();
    assert_eq!(parsed.result, vec!["dataset1", "dataset2"]);
}

#[test]
fn test_parse_package_show_response_none() {
    // Test that a null result is handled as None (no dataset metadata returned).
    let data = serde_json::json!({ "result": null });
    let parsed: PackageShowResponse = serde_json::from_value(data).unwrap();
    assert!(parsed.result.is_none());
}

#[test]
fn test_parse_package_show_response_some() {
    // Test that a valid result is parsed and fields are accessible.
    // This simulates a real CKAN package_show response with all required fields.
    let data = serde_json::json!({
        "result": {
            "id": "abc",
            "title": "Test",
            "notes": "desc",
            "license_title": "Open",
            "organization": { "title": "Org" },
            "metadata_created": "2020-01-01",
            "metadata_modified": "2020-01-02",
            "resources": []
        }
    });
    let parsed: PackageShowResponse = serde_json::from_value(data).unwrap();
    assert!(parsed.result.is_some());
    let result = parsed.result.unwrap();
    assert_eq!(result.id, "abc");
    assert_eq!(result.title, "Test");
    // Additional checks can be added for other fields if needed.
}

#[test]
fn test_config_validation_valid() {
    // Test that a valid configuration passes validation.
    let config = Config::new();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_empty_bucket() {
    // Test that empty bucket name fails validation with appropriate error.
    let mut config = Config::new();
    config.bucket_name = "".to_string();
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("S3 bucket name must not be empty"));
}

#[test]
fn test_config_validation_zero_concurrency() {
    // Test that zero concurrency limit fails validation.
    let mut config = Config::new();
    config.concurrency_limit = 0;
    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Concurrency limit must be greater than zero"));
}
