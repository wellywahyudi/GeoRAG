//! Test format metadata serialization and deserialization

use georag_core::models::dataset::{Dataset, DatasetId, FormatMetadata, GeometryType};
use chrono::Utc;
use std::path::PathBuf;

#[test]
fn test_format_metadata_serialization() {
    // Create a dataset with format metadata
    let dataset = Dataset {
        id: DatasetId(1),
        name: "test_dataset".to_string(),
        path: PathBuf::from("/tmp/test.geojson"),
        geometry_type: GeometryType::Point,
        feature_count: 100,
        crs: 4326,
        format: FormatMetadata {
            format_name: "GeoJSON".to_string(),
            format_version: Some("1.0".to_string()),
            layer_name: None,
            page_count: None,
            paragraph_count: None,
            extraction_method: None,
        },
        added_at: Utc::now(),
    };

    // Test serialization
    let json = serde_json::to_string(&dataset).expect("Failed to serialize");
    assert!(json.contains("GeoJSON"));
    assert!(json.contains("format_name"));

    // Test deserialization
    let deserialized: Dataset = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.format.format_name, "GeoJSON");
    assert_eq!(deserialized.format.format_version, Some("1.0".to_string()));
    assert_eq!(deserialized.format.layer_name, None);
    assert_eq!(deserialized.format.page_count, None);
}

#[test]
fn test_format_metadata_with_all_fields() {
    // Create format metadata with all optional fields populated
    let format = FormatMetadata {
        format_name: "GeoPackage".to_string(),
        format_version: Some("1.2".to_string()),
        layer_name: Some("buildings".to_string()),
        page_count: Some(10),
        paragraph_count: Some(50),
        extraction_method: Some("GDAL".to_string()),
    };

    // Test serialization
    let json = serde_json::to_string(&format).expect("Failed to serialize");
    
    // Test deserialization
    let deserialized: FormatMetadata = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.format_name, "GeoPackage");
    assert_eq!(deserialized.format_version, Some("1.2".to_string()));
    assert_eq!(deserialized.layer_name, Some("buildings".to_string()));
    assert_eq!(deserialized.page_count, Some(10));
    assert_eq!(deserialized.paragraph_count, Some(50));
    assert_eq!(deserialized.extraction_method, Some("GDAL".to_string()));
}

#[test]
fn test_format_metadata_document_format() {
    // Create format metadata for a document format
    let format = FormatMetadata {
        format_name: "PDF".to_string(),
        format_version: None,
        layer_name: None,
        page_count: Some(25),
        paragraph_count: Some(150),
        extraction_method: Some("pdf-extract".to_string()),
    };

    // Test serialization
    let json = serde_json::to_string(&format).expect("Failed to serialize");
    
    // Test deserialization
    let deserialized: FormatMetadata = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.format_name, "PDF");
    assert_eq!(deserialized.page_count, Some(25));
    assert_eq!(deserialized.paragraph_count, Some(150));
    assert_eq!(deserialized.extraction_method, Some("pdf-extract".to_string()));
}
