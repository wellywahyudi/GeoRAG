//! Test format metadata serialization and deserialization

use chrono::Utc;
use georag_core::models::dataset::{Dataset, DatasetId, FormatMetadata};
use georag_core::models::GeometryType;
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
            spatial_association: None,
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
        spatial_association: None,
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
        spatial_association: None,
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

#[test]
fn test_format_metadata_docx_format() {
    // Create format metadata for a DOCX document format
    let format = FormatMetadata {
        format_name: "DOCX".to_string(),
        format_version: None,
        layer_name: None,
        page_count: None,
        paragraph_count: Some(42),
        extraction_method: Some("docx-rs".to_string()),
        spatial_association: None,
    };

    // Test serialization
    let json = serde_json::to_string(&format).expect("Failed to serialize");

    // Test deserialization
    let deserialized: FormatMetadata = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.format_name, "DOCX");
    assert_eq!(deserialized.page_count, None);
    assert_eq!(deserialized.paragraph_count, Some(42));
    assert_eq!(deserialized.extraction_method, Some("docx-rs".to_string()));
}

#[test]
fn test_format_metadata_with_spatial_association() {
    use georag_core::models::dataset::SpatialAssociation;
    use std::path::PathBuf;

    // Create format metadata with spatial association
    let format = FormatMetadata {
        format_name: "PDF".to_string(),
        format_version: None,
        layer_name: None,
        page_count: Some(10),
        paragraph_count: Some(50),
        extraction_method: Some("pdf-extract".to_string()),
        spatial_association: Some(SpatialAssociation {
            source: "manual".to_string(),
            geometry_file: Some(PathBuf::from("/path/to/geometry.geojson")),
            associated_at: Utc::now(),
            description: Some("Manually associated with building location".to_string()),
        }),
    };

    // Test serialization
    let json = serde_json::to_string(&format).expect("Failed to serialize");

    // Test deserialization
    let deserialized: FormatMetadata = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.format_name, "PDF");
    assert!(deserialized.spatial_association.is_some());

    let spatial_assoc = deserialized.spatial_association.unwrap();
    assert_eq!(spatial_assoc.source, "manual");
    assert_eq!(spatial_assoc.geometry_file, Some(PathBuf::from("/path/to/geometry.geojson")));
    assert_eq!(
        spatial_assoc.description,
        Some("Manually associated with building location".to_string())
    );
}

#[test]
fn test_feature_with_null_geometry() {
    use georag_core::models::query::{Feature, FeatureId};
    use std::collections::HashMap;

    // Create a feature without geometry (document)
    let feature = Feature::without_geometry(
        FeatureId(1),
        HashMap::from([
            ("content".to_string(), serde_json::json!("Document text")),
            ("format".to_string(), serde_json::json!("PDF")),
        ]),
        4326,
    );

    assert!(!feature.has_geometry());
    assert!(!feature.is_spatially_queryable());

    // Test serialization
    let json = serde_json::to_string(&feature).expect("Failed to serialize");

    // Test deserialization
    let deserialized: Feature = serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(!deserialized.has_geometry());
    assert_eq!(deserialized.properties.get("content").unwrap(), "Document text");
}

#[test]
fn test_feature_geometry_association() {
    use georag_core::models::query::{Feature, FeatureId};
    use georag_core::models::Geometry;
    use std::collections::HashMap;

    // Create a feature without geometry
    let mut feature = Feature::without_geometry(
        FeatureId(1),
        HashMap::from([("content".to_string(), serde_json::json!("Document text"))]),
        4326,
    );

    assert!(!feature.has_geometry());

    // Associate geometry using typed Geometry
    let geometry = Geometry::point(-122.4, 47.6);
    feature.associate_geometry(geometry);

    assert!(feature.has_geometry());
    assert!(feature.is_spatially_queryable());

    // Verify geometry is correct
    let geom = feature.geometry.as_ref().unwrap();
    assert!(matches!(geom, Geometry::Point { .. }));
}
