//! GeoJSON format reader implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{GeoragError, Result};
use crate::formats::{FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation};
use crate::formats::validation::FormatValidator;

/// GeoJSON format reader
pub struct GeoJsonReader;

#[async_trait]
impl FormatReader for GeoJsonReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        // Read the file
        let content = fs::read_to_string(path)
            .map_err(|e| GeoragError::Io(e))?;

        // Parse as GeoJSON
        let geojson: geojson::GeoJson = content.parse()
            .map_err(|e| GeoragError::FormatValidation {
                format: "GeoJSON".to_string(),
                reason: format!("Failed to parse GeoJSON: {}", e),
            })?;

        // Extract features and metadata
        let (features, crs) = self.extract_features_and_crs(&geojson)?;

        // Get dataset name from filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        Ok(FormatDataset {
            name,
            format_metadata: FormatMetadata {
                format_name: "GeoJSON".to_string(),
                format_version: None,
                layer_name: None,
                page_count: None,
                paragraph_count: None,
                extraction_method: None,
                spatial_association: None,
            },
            crs,
            features,
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["json", "geojson"]
    }

    fn format_name(&self) -> &str {
        "GeoJSON"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        // Basic file validation
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Validate JSON structure
        let json_validation = FormatValidator::validate_json_structure(path);
        
        // If JSON is valid, try to parse as GeoJSON
        if json_validation.is_valid() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    if let Err(e) = content.parse::<geojson::GeoJson>() {
                        validation.errors.push(format!("Invalid GeoJSON: {}", e));
                    }
                }
                Err(e) => {
                    validation.errors.push(format!("Cannot read file: {}", e));
                }
            }
        }

        // Merge validations
        Ok(FormatValidator::merge_validations(vec![validation, json_validation]))
    }
}

impl GeoJsonReader {
    /// Extract features and CRS from GeoJSON
    fn extract_features_and_crs(&self, geojson: &geojson::GeoJson) -> Result<(Vec<FormatFeature>, u32)> {
        match geojson {
            geojson::GeoJson::FeatureCollection(fc) => {
                let features = fc.features
                    .iter()
                    .enumerate()
                    .map(|(idx, feature)| self.convert_feature(feature, idx))
                    .collect();

                // Extract CRS (default to WGS84 if not specified)
                let crs = fc.foreign_members
                    .as_ref()
                    .and_then(|fm| fm.get("crs"))
                    .and_then(|crs_obj| extract_epsg_from_crs(crs_obj))
                    .unwrap_or(4326);

                Ok((features, crs))
            }
            geojson::GeoJson::Feature(feature) => {
                let features = vec![self.convert_feature(feature, 0)];
                Ok((features, 4326))
            }
            geojson::GeoJson::Geometry(geom) => {
                // Single geometry - wrap in a feature
                let geometry_json = serde_json::to_value(geom)
                    .map_err(|e| GeoragError::Serialization(format!("Failed to serialize geometry: {}", e)))?;

                let feature = FormatFeature {
                    id: "0".to_string(),
                    geometry: Some(geometry_json),
                    properties: HashMap::new(),
                };

                Ok((vec![feature], 4326))
            }
        }
    }

    /// Convert a GeoJSON feature to FormatFeature
    fn convert_feature(&self, feature: &geojson::Feature, idx: usize) -> FormatFeature {
        // Get feature ID (use index if not present)
        let id = feature.id
            .as_ref()
            .map(|id| match id {
                geojson::feature::Id::String(s) => s.clone(),
                geojson::feature::Id::Number(n) => n.to_string(),
            })
            .unwrap_or_else(|| idx.to_string());

        // Convert geometry to JSON value
        let geometry = feature.geometry
            .as_ref()
            .and_then(|geom| serde_json::to_value(geom).ok());

        // Convert properties from serde_json::Map to HashMap
        let properties = feature.properties
            .as_ref()
            .map(|props| props.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        FormatFeature {
            id,
            geometry,
            properties,
        }
    }
}

/// Extract EPSG code from CRS object
fn extract_epsg_from_crs(crs: &serde_json::Value) -> Option<u32> {
    // Try to extract from properties.name
    if let Some(props) = crs.get("properties") {
        if let Some(name) = props.get("name") {
            if let Some(name_str) = name.as_str() {
                // Parse "EPSG:4326" or "urn:ogc:def:crs:EPSG::4326"
                if let Some(epsg_str) = name_str.split(':').last() {
                    return epsg_str.parse().ok();
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_geojson_reader_feature_collection() {
        let reader = GeoJsonReader;
        
        // Create a temporary GeoJSON file
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.geojson");
        
        let geojson_content = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "id": "feature1",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [0.0, 0.0]
                    },
                    "properties": {
                        "name": "Test Point"
                    }
                }
            ]
        }"#;
        
        fs::write(&file_path, geojson_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.name, "test");
        assert_eq!(result.format_metadata.format_name, "GeoJSON");
        assert_eq!(result.crs, 4326);
        assert_eq!(result.features.len(), 1);
        assert_eq!(result.features[0].id, "feature1");
    }

    #[tokio::test]
    async fn test_geojson_reader_single_feature() {
        let reader = GeoJsonReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.geojson");
        
        let geojson_content = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [1.0, 2.0]
            },
            "properties": {
                "name": "Single Feature"
            }
        }"#;
        
        fs::write(&file_path, geojson_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.features.len(), 1);
        assert!(result.features[0].geometry.is_some());
    }

    #[tokio::test]
    async fn test_geojson_reader_validation() {
        let reader = GeoJsonReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("invalid.geojson");
        
        // Write invalid JSON
        fs::write(&file_path, "not valid json").unwrap();
        
        let validation = reader.validate(&file_path).await.unwrap();
        
        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_supported_extensions() {
        let reader = GeoJsonReader;
        assert_eq!(reader.supported_extensions(), &["json", "geojson"]);
    }

    #[test]
    fn test_format_name() {
        let reader = GeoJsonReader;
        assert_eq!(reader.format_name(), "GeoJSON");
    }
}
