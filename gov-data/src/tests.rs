// Unit tests for gov-data (moved from main.rs)
use super::*;
use serde_json::json;

#[test]
fn test_parse_package_list_response() {
    // Test that a valid package list response is parsed into the struct.
    let data = json!({ "result": ["dataset1", "dataset2"] });
    let parsed: PackageListResponse = serde_json::from_value(data).unwrap();
    assert_eq!(parsed.result, vec!["dataset1", "dataset2"]);
}

#[test]
fn test_parse_package_show_response_none() {
    // Test that a null result is handled as None.
    let data = json!({ "result": null });
    let parsed: PackageShowResponse = serde_json::from_value(data).unwrap();
    assert!(parsed.result.is_none());
}

#[test]
fn test_parse_package_show_response_some() {
    // Test that a valid result is parsed and fields are accessible.
    let data = json!({ "result": { "id": "abc", "title": "Test", "notes": "desc", "license_title": "Open", "organization": { "title": "Org" }, "metadata_created": "2020-01-01", "metadata_modified": "2020-01-02", "resources": [] } });
    let parsed: PackageShowResponse = serde_json::from_value(data).unwrap();
    assert!(parsed.result.is_some());
    let result = parsed.result.unwrap();
    assert_eq!(result["id"], "abc");
    assert_eq!(result["title"], "Test");
} 