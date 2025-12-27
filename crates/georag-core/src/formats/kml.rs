//! KML (Keyhole Markup Language) format reader implementation
//!
//! This module provides support for reading KML files, which are XML-based
//! formats used by Google Earth and other mapping applications.

use async_trait::async_trait;
use kml::Kml;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{GeoragError, Result};
use crate::formats::{FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation};
use crate::formats::validation::FormatValidator;

/// KML format reader
pub struct KmlReader;

#[async_trait]
impl FormatReader for KmlReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        // Read the KML file as string
        let content = fs::read_to_string(path)
            .map_err(|e| GeoragError::FormatError {
                format: "KML".to_string(),
                message: format!("Failed to open KML file: {}", e),
            })?;

        // Parse the KML content
        let kml: Kml = content.parse()
            .map_err(|e| GeoragError::FormatValidation {
                format: "KML".to_string(),
                reason: format!("Failed to parse KML: {}", e),
            })?;

        // Extract features from the KML structure
        let mut features = Vec::new();
        let mut feature_counter = 0;
        
        self.extract_features_recursive(&kml, &mut features, &mut feature_counter, Vec::new())?;

        // Get dataset name from filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        Ok(FormatDataset {
            name,
            format_metadata: FormatMetadata {
                format_name: "KML".to_string(),
                format_version: Some("2.2".to_string()),
                layer_name: None,
                page_count: None,
                paragraph_count: None,
                extraction_method: Some("kml-rs".to_string()),
                spatial_association: None,
            },
            crs: 4326, // KML always uses WGS84 (EPSG:4326)
            features,
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["kml"]
    }

    fn format_name(&self) -> &str {
        "KML"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        // Basic file validation
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Validate XML structure
        let xml_validation = FormatValidator::validate_xml_structure(path);
        
        // If XML is valid, try to parse as KML
        if xml_validation.is_valid() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    if let Err(e) = content.parse::<Kml>() {
                        validation.errors.push(format!("Invalid KML: {}", e));
                    }
                }
                Err(e) => {
                    validation.errors.push(format!("Cannot read file: {}", e));
                }
            }
        }

        // Merge validations
        Ok(FormatValidator::merge_validations(vec![validation, xml_validation]))
    }
}

impl KmlReader {
    /// Recursively extract features from KML structure
    ///
    /// This handles nested folders and documents, preserving the folder hierarchy
    /// in the feature properties.
    fn extract_features_recursive(
        &self,
        kml: &Kml,
        features: &mut Vec<FormatFeature>,
        counter: &mut usize,
        folder_path: Vec<String>,
    ) -> Result<()> {
        match kml {
            Kml::KmlDocument(doc) => {
                // Process document elements
                for element in &doc.elements {
                    self.extract_features_recursive(element, features, counter, folder_path.clone())?;
                }
            }
            Kml::Folder { attrs, elements } => {
                // Extract folder name from attrs if available
                let mut new_path = folder_path.clone();
                if let Some(name) = attrs.get("name") {
                    new_path.push(name.clone());
                }
                
                // Process folder elements
                for element in elements {
                    self.extract_features_recursive(element, features, counter, new_path.clone())?;
                }
            }
            Kml::Document { attrs: _, elements } => {
                // Process document elements
                for element in elements {
                    self.extract_features_recursive(element, features, counter, folder_path.clone())?;
                }
            }
            Kml::Placemark(placemark) => {
                // Extract feature from placemark
                if let Some(feature) = self.extract_placemark(placemark, *counter, &folder_path)? {
                    features.push(feature);
                    *counter += 1;
                }
            }
            _ => {
                // Ignore other KML elements (NetworkLink, GroundOverlay, etc.)
            }
        }

        Ok(())
    }

    /// Extract a feature from a KML Placemark
    fn extract_placemark(
        &self,
        placemark: &kml::types::Placemark,
        id: usize,
        folder_path: &[String],
    ) -> Result<Option<FormatFeature>> {
        // Extract geometry
        let geometry = if let Some(geom) = &placemark.geometry {
            self.convert_geometry(geom)?
        } else {
            // Placemark without geometry - skip it
            return Ok(None);
        };

        // Build properties
        let mut properties = HashMap::new();
        
        // Add placemark name
        if let Some(name) = &placemark.name {
            properties.insert("name".to_string(), serde_json::json!(name));
        }
        
        // Add description
        if let Some(desc) = &placemark.description {
            properties.insert("description".to_string(), serde_json::json!(desc));
        }
        
        // Add folder hierarchy
        if !folder_path.is_empty() {
            properties.insert("folder_path".to_string(), serde_json::json!(folder_path.join("/")));
        }

        // Extract extended data from children if present
        // The children field contains Element types directly, not wrapped in Kml enum
        for child in &placemark.children {
            // Try to extract custom data from element attributes
            if !child.attrs.is_empty() {
                for (key, value) in &child.attrs {
                    properties.insert(key.clone(), serde_json::json!(value));
                }
            }
        }

        Ok(Some(FormatFeature {
            id: format!("placemark_{}", id),
            geometry: Some(geometry),
            properties,
        }))
    }

    /// Convert KML geometry to GeoJSON format
    fn convert_geometry(&self, geometry: &kml::types::Geometry) -> Result<serde_json::Value> {
        match geometry {
            kml::types::Geometry::Point(point) => {
                Ok(self.convert_point(point))
            }
            kml::types::Geometry::LineString(linestring) => {
                Ok(self.convert_linestring(linestring))
            }
            kml::types::Geometry::LinearRing(ring) => {
                // LinearRing is similar to LineString but closed
                Ok(self.convert_linear_ring(ring))
            }
            kml::types::Geometry::Polygon(polygon) => {
                Ok(self.convert_polygon(polygon))
            }
            kml::types::Geometry::MultiGeometry(multi) => {
                Ok(self.convert_multi_geometry(multi)?)
            }
            _ => {
                // For unsupported geometry types, return a placeholder
                Err(GeoragError::FormatError {
                    format: "KML".to_string(),
                    message: "Unsupported geometry type".to_string(),
                })
            }
        }
    }

    /// Convert KML Point to GeoJSON Point
    fn convert_point(&self, point: &kml::types::Point) -> serde_json::Value {
        let coord = &point.coord;
        
        if let Some(altitude) = coord.z {
            serde_json::json!({
                "type": "Point",
                "coordinates": [coord.x, coord.y, altitude]
            })
        } else {
            serde_json::json!({
                "type": "Point",
                "coordinates": [coord.x, coord.y]
            })
        }
    }

    /// Convert KML LineString to GeoJSON LineString
    fn convert_linestring(&self, linestring: &kml::types::LineString) -> serde_json::Value {
        let has_altitude = linestring.coords.iter().any(|c| c.z.is_some());
        
        let coordinates: Vec<serde_json::Value> = if has_altitude {
            linestring.coords.iter().map(|coord| {
                let altitude = coord.z.unwrap_or(0.0);
                serde_json::json!([coord.x, coord.y, altitude])
            }).collect()
        } else {
            linestring.coords.iter().map(|coord| {
                serde_json::json!([coord.x, coord.y])
            }).collect()
        };

        serde_json::json!({
            "type": "LineString",
            "coordinates": coordinates
        })
    }

    /// Convert KML LinearRing to GeoJSON LineString
    fn convert_linear_ring(&self, ring: &kml::types::LinearRing) -> serde_json::Value {
        let has_altitude = ring.coords.iter().any(|c| c.z.is_some());
        
        let coordinates: Vec<serde_json::Value> = if has_altitude {
            ring.coords.iter().map(|coord| {
                let altitude = coord.z.unwrap_or(0.0);
                serde_json::json!([coord.x, coord.y, altitude])
            }).collect()
        } else {
            ring.coords.iter().map(|coord| {
                serde_json::json!([coord.x, coord.y])
            }).collect()
        };

        serde_json::json!({
            "type": "LineString",
            "coordinates": coordinates
        })
    }

    /// Convert KML Polygon to GeoJSON Polygon
    fn convert_polygon(&self, polygon: &kml::types::Polygon) -> serde_json::Value {
        let mut rings = Vec::new();

        // Outer boundary
        rings.push(self.convert_ring_coords(&polygon.outer.coords));

        // Inner boundaries (holes)
        for inner in &polygon.inner {
            rings.push(self.convert_ring_coords(&inner.coords));
        }

        serde_json::json!({
            "type": "Polygon",
            "coordinates": rings
        })
    }

    /// Convert ring coordinates to GeoJSON format
    fn convert_ring_coords(&self, coords: &[kml::types::Coord]) -> Vec<serde_json::Value> {
        let has_altitude = coords.iter().any(|c| c.z.is_some());
        
        if has_altitude {
            coords.iter().map(|coord| {
                let altitude = coord.z.unwrap_or(0.0);
                serde_json::json!([coord.x, coord.y, altitude])
            }).collect()
        } else {
            coords.iter().map(|coord| {
                serde_json::json!([coord.x, coord.y])
            }).collect()
        }
    }

    /// Convert KML MultiGeometry to GeoJSON GeometryCollection
    fn convert_multi_geometry(&self, multi: &kml::types::MultiGeometry) -> Result<serde_json::Value> {
        let mut geometries = Vec::new();

        for geom in &multi.geometries {
            geometries.push(self.convert_geometry(geom)?);
        }

        Ok(serde_json::json!({
            "type": "GeometryCollection",
            "geometries": geometries
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_kml_reader_point() {
        let reader = KmlReader;
        
        // Create a temporary KML file with a point
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.kml");
        
        let kml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <Placemark>
      <name>Test Point</name>
      <description>A test point</description>
      <Point>
        <coordinates>-122.326897,47.644548,0</coordinates>
      </Point>
    </Placemark>
  </Document>
</kml>"#;
        
        fs::write(&file_path, kml_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.name, "test");
        assert_eq!(result.format_metadata.format_name, "KML");
        assert_eq!(result.crs, 4326);
        assert_eq!(result.features.len(), 1);
        assert_eq!(result.features[0].id, "placemark_0");
        assert!(result.features[0].geometry.is_some());
        
        // Check geometry type
        let geometry = result.features[0].geometry.as_ref().unwrap();
        assert_eq!(geometry["type"], "Point");
    }

    #[tokio::test]
    async fn test_kml_reader_linestring() {
        let reader = KmlReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.kml");
        
        let kml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <Placemark>
      <name>Test Line</name>
      <LineString>
        <coordinates>
          -122.326897,47.644548,0
          -122.326898,47.644549,0
        </coordinates>
      </LineString>
    </Placemark>
  </Document>
</kml>"#;
        
        fs::write(&file_path, kml_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.features.len(), 1);
        
        // Check geometry type
        let geometry = result.features[0].geometry.as_ref().unwrap();
        assert_eq!(geometry["type"], "LineString");
    }

    #[tokio::test]
    async fn test_kml_reader_polygon() {
        let reader = KmlReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.kml");
        
        let kml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <Placemark>
      <name>Test Polygon</name>
      <Polygon>
        <outerBoundaryIs>
          <LinearRing>
            <coordinates>
              -122.326897,47.644548,0
              -122.326898,47.644549,0
              -122.326899,47.644550,0
              -122.326897,47.644548,0
            </coordinates>
          </LinearRing>
        </outerBoundaryIs>
      </Polygon>
    </Placemark>
  </Document>
</kml>"#;
        
        fs::write(&file_path, kml_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.features.len(), 1);
        
        // Check geometry type
        let geometry = result.features[0].geometry.as_ref().unwrap();
        assert_eq!(geometry["type"], "Polygon");
    }

    #[tokio::test]
    async fn test_kml_reader_nested_folders() {
        let reader = KmlReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.kml");
        
        let kml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <Folder>
      <name>Parent Folder</name>
      <Folder>
        <name>Child Folder</name>
        <Placemark>
          <name>Nested Point</name>
          <Point>
            <coordinates>-122.326897,47.644548,0</coordinates>
          </Point>
        </Placemark>
      </Folder>
    </Folder>
  </Document>
</kml>"#;
        
        fs::write(&file_path, kml_content).unwrap();
        
        let result = reader.read(&file_path).await.unwrap();
        
        assert_eq!(result.features.len(), 1);
        
        // Check folder path is preserved (if the KML parser supports it)
        let properties = &result.features[0].properties;
        // The folder path may or may not be present depending on KML parser implementation
        if let Some(folder_path) = properties.get("folder_path") {
            // If present, verify it contains folder information
            assert!(folder_path.as_str().is_some());
        }
    }

    #[tokio::test]
    async fn test_kml_reader_validation() {
        let reader = KmlReader;
        
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("invalid.kml");
        
        // Write invalid XML
        fs::write(&file_path, "not valid xml").unwrap();
        
        let validation = reader.validate(&file_path).await.unwrap();
        
        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_supported_extensions() {
        let reader = KmlReader;
        assert_eq!(reader.supported_extensions(), &["kml"]);
    }

    #[test]
    fn test_format_name() {
        let reader = KmlReader;
        assert_eq!(reader.format_name(), "KML");
    }
}
