//! Shapefile format reader implementation
//!
//! This module provides support for reading ESRI Shapefiles using pure Rust.
//! Shapefiles consist of multiple component files (.shp, .shx, .dbf, .prj)
//! that must all be present for proper reading.

use async_trait::async_trait;
use shapefile::dbase::FieldValue as DbaseFieldValue;
use shapefile::{Reader as ShapefileReader, Shape};
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

use crate::error::{GeoragError, Result};
use crate::formats::{FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation};
use crate::formats::validation::FormatValidator;

/// Shapefile format reader
pub struct ShapefileFormatReader;

#[async_trait]
impl FormatReader for ShapefileFormatReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        // Verify all required component files exist
        self.verify_components(path)?;

        // Open the Shapefile
        let mut reader = ShapefileReader::from_path(path)
            .map_err(|e| GeoragError::FormatError {
                format: "Shapefile".to_string(),
                message: format!("Failed to open Shapefile: {}", e),
            })?;

        // Extract CRS
        let crs = self.extract_crs(path)?;

        // Read features
        let features = self.read_features(&mut reader)?;

        // Get dataset name from filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        Ok(FormatDataset {
            name,
            format_metadata: FormatMetadata {
                format_name: "Shapefile".to_string(),
                format_version: None,
                layer_name: None,
                page_count: None,
                paragraph_count: None,
                extraction_method: Some("shapefile-rs".to_string()),
                spatial_association: None,
            },
            crs,
            features,
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["shp"]
    }

    fn format_name(&self) -> &str {
        "Shapefile"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        // Basic file validation
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Check for required component files
        let base = match self.get_shapefile_base(path) {
            Ok(b) => b,
            Err(e) => {
                validation.errors.push(format!("Invalid Shapefile path: {}", e));
                return Ok(validation);
            }
        };

        // Validate component files using centralized validator
        let component_validation = FormatValidator::validate_component_files(
            &base,
            &["shp", "shx", "dbf"],
            &["prj"],
        );

        // Merge validations
        Ok(FormatValidator::merge_validations(vec![validation, component_validation]))
    }
}

impl ShapefileFormatReader {
    /// Get the base path for a Shapefile (without extension)
    fn get_shapefile_base(&self, path: &Path) -> Result<std::path::PathBuf> {
        if !self.has_extension(path, "shp") {
            return Err(GeoragError::InvalidPath {
                path: path.to_path_buf(),
                reason: "Not a Shapefile (.shp)".to_string(),
            });
        }
        
        Ok(path.with_extension(""))
    }

    /// Check if a path has a specific extension
    fn has_extension(&self, path: &Path, ext: &str) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
    }

    /// Verify that all required Shapefile component files exist
    fn verify_components(&self, path: &Path) -> Result<()> {
        let base = self.get_shapefile_base(path)?;
        let required_extensions = vec!["shp", "shx", "dbf"];
        let mut missing = Vec::new();
        
        for ext in required_extensions {
            let component_path = base.with_extension(ext);
            if !component_path.exists() {
                missing.push(format!(".{}", ext));
            }
        }
        
        if !missing.is_empty() {
            return Err(GeoragError::FormatError {
                format: "Shapefile".to_string(),
                message: format!("Missing required component files: {}", missing.join(", ")),
            });
        }
        
        Ok(())
    }

    /// Extract CRS from the Shapefile .prj file
    fn extract_crs(&self, path: &Path) -> Result<u32> {
        let base = self.get_shapefile_base(path)?;
        let prj_path = base.with_extension("prj");

        if !prj_path.exists() {
            // No .prj file, default to EPSG:4326
            return Ok(4326);
        }

        // Read .prj file content
        let prj_content = fs::read_to_string(&prj_path)
            .map_err(|e| GeoragError::FormatError {
                format: "Shapefile".to_string(),
                message: format!("Failed to read .prj file: {}", e),
            })?;

        // Try to parse WKT and extract EPSG code
        if let Some(epsg) = self.parse_epsg_from_wkt(&prj_content) {
            return Ok(epsg);
        }

        // Try using wkt crate to parse
        match wkt::Wkt::<f64>::from_str(&prj_content) {
            Ok(_wkt) => {
                // Successfully parsed WKT, but couldn't extract EPSG
                // Try to find AUTHORITY in the string
                if let Some(epsg) = self.extract_authority_code(&prj_content) {
                    return Ok(epsg);
                }
            }
            Err(_) => {
                // Failed to parse WKT
            }
        }

        // Could not extract EPSG code, default to 4326
        Ok(4326)
    }

    /// Parse EPSG code from WKT string
    fn parse_epsg_from_wkt(&self, wkt: &str) -> Option<u32> {
        // Look for AUTHORITY["EPSG","4326"] pattern
        if let Some(start) = wkt.find("AUTHORITY[\"EPSG\",\"") {
            let code_start = start + 18; // Length of 'AUTHORITY["EPSG","'
            if let Some(end) = wkt[code_start..].find('\"') {
                if let Ok(code) = wkt[code_start..code_start + end].parse::<u32>() {
                    return Some(code);
                }
            }
        }

        // Look for EPSG: prefix
        if let Some(start) = wkt.find("EPSG:") {
            let code_start = start + 5;
            let code_str: String = wkt[code_start..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(code) = code_str.parse::<u32>() {
                return Some(code);
            }
        }

        None
    }

    /// Extract AUTHORITY code from WKT string
    fn extract_authority_code(&self, wkt: &str) -> Option<u32> {
        // Look for AUTHORITY["EPSG","code"] anywhere in the string
        for line in wkt.lines() {
            if line.contains("AUTHORITY") && line.contains("EPSG") {
                // Extract the number after EPSG
                if let Some(start) = line.find("EPSG") {
                    let after_epsg = &line[start + 4..];
                    // Find the first sequence of digits
                    let digits: String = after_epsg
                        .chars()
                        .skip_while(|c| !c.is_ascii_digit())
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(code) = digits.parse::<u32>() {
                        return Some(code);
                    }
                }
            }
        }
        None
    }

    /// Read features from the Shapefile
    fn read_features(&self, reader: &mut shapefile::Reader<BufReader<fs::File>, BufReader<fs::File>>) -> Result<Vec<FormatFeature>> {
        let mut features = Vec::new();

        // Iterate through all shapes and records
        for result in reader.iter_shapes_and_records() {
            let (shape, record) = result.map_err(|e| GeoragError::FormatError {
                format: "Shapefile".to_string(),
                message: format!("Failed to read feature: {}", e),
            })?;

            // Convert shape to GeoJSON geometry
            let geometry = self.convert_shape_to_geojson(&shape)?;

            // Extract properties from DBF record
            let properties = self.extract_properties(&record)?;

            // Generate feature ID from record number
            let id = features.len().to_string();

            features.push(FormatFeature {
                id,
                geometry: Some(geometry),
                properties,
            });
        }

        Ok(features)
    }

    /// Convert shapefile Shape to GeoJSON Value
    fn convert_shape_to_geojson(&self, shape: &Shape) -> Result<serde_json::Value> {
        match shape {
            Shape::Point(point) => {
                Ok(serde_json::json!({
                    "type": "Point",
                    "coordinates": [point.x, point.y]
                }))
            }
            Shape::PointZ(point) => {
                Ok(serde_json::json!({
                    "type": "Point",
                    "coordinates": [point.x, point.y, point.z]
                }))
            }
            Shape::PointM(point) => {
                Ok(serde_json::json!({
                    "type": "Point",
                    "coordinates": [point.x, point.y]
                }))
            }
            Shape::Polyline(polyline) => {
                let coordinates: Vec<Vec<[f64; 2]>> = polyline
                    .parts()
                    .into_iter()
                    .map(|part| {
                        part.iter()
                            .map(|p| [p.x, p.y])
                            .collect()
                    })
                    .collect();

                if coordinates.len() == 1 {
                    Ok(serde_json::json!({
                        "type": "LineString",
                        "coordinates": coordinates[0]
                    }))
                } else {
                    Ok(serde_json::json!({
                        "type": "MultiLineString",
                        "coordinates": coordinates
                    }))
                }
            }
            Shape::PolylineZ(polyline) => {
                let coordinates: Vec<Vec<[f64; 3]>> = polyline
                    .parts()
                    .into_iter()
                    .map(|part| {
                        part.iter()
                            .map(|p| [p.x, p.y, p.z])
                            .collect()
                    })
                    .collect();

                if coordinates.len() == 1 {
                    Ok(serde_json::json!({
                        "type": "LineString",
                        "coordinates": coordinates[0]
                    }))
                } else {
                    Ok(serde_json::json!({
                        "type": "MultiLineString",
                        "coordinates": coordinates
                    }))
                }
            }
            Shape::PolylineM(polyline) => {
                let coordinates: Vec<Vec<[f64; 2]>> = polyline
                    .parts()
                    .into_iter()
                    .map(|part| {
                        part.iter()
                            .map(|p| [p.x, p.y])
                            .collect()
                    })
                    .collect();

                if coordinates.len() == 1 {
                    Ok(serde_json::json!({
                        "type": "LineString",
                        "coordinates": coordinates[0]
                    }))
                } else {
                    Ok(serde_json::json!({
                        "type": "MultiLineString",
                        "coordinates": coordinates
                    }))
                }
            }
            Shape::Polygon(polygon) => {
                let rings: Vec<Vec<[f64; 2]>> = polygon
                    .rings()
                    .into_iter()
                    .map(|ring| {
                        ring.points()
                            .iter()
                            .map(|p| [p.x, p.y])
                            .collect()
                    })
                    .collect();

                Ok(serde_json::json!({
                    "type": "Polygon",
                    "coordinates": rings
                }))
            }
            Shape::PolygonZ(polygon) => {
                let rings: Vec<Vec<[f64; 3]>> = polygon
                    .rings()
                    .into_iter()
                    .map(|ring| {
                        ring.points()
                            .iter()
                            .map(|p| [p.x, p.y, p.z])
                            .collect()
                    })
                    .collect();

                Ok(serde_json::json!({
                    "type": "Polygon",
                    "coordinates": rings
                }))
            }
            Shape::PolygonM(polygon) => {
                let rings: Vec<Vec<[f64; 2]>> = polygon
                    .rings()
                    .into_iter()
                    .map(|ring| {
                        ring.points()
                            .iter()
                            .map(|p| [p.x, p.y])
                            .collect()
                    })
                    .collect();

                Ok(serde_json::json!({
                    "type": "Polygon",
                    "coordinates": rings
                }))
            }
            Shape::Multipoint(multipoint) => {
                let coordinates: Vec<[f64; 2]> = multipoint
                    .points()
                    .iter()
                    .map(|p| [p.x, p.y])
                    .collect();

                Ok(serde_json::json!({
                    "type": "MultiPoint",
                    "coordinates": coordinates
                }))
            }
            Shape::MultipointZ(multipoint) => {
                let coordinates: Vec<[f64; 3]> = multipoint
                    .points()
                    .iter()
                    .map(|p| [p.x, p.y, p.z])
                    .collect();

                Ok(serde_json::json!({
                    "type": "MultiPoint",
                    "coordinates": coordinates
                }))
            }
            Shape::MultipointM(multipoint) => {
                let coordinates: Vec<[f64; 2]> = multipoint
                    .points()
                    .iter()
                    .map(|p| [p.x, p.y])
                    .collect();

                Ok(serde_json::json!({
                    "type": "MultiPoint",
                    "coordinates": coordinates
                }))
            }
            Shape::Multipatch(_) => {
                // Multipatch is a complex 3D shape type, not commonly used
                Err(GeoragError::FormatError {
                    format: "Shapefile".to_string(),
                    message: "Multipatch geometry type is not supported".to_string(),
                })
            }
            Shape::NullShape => {
                // Null shape - no geometry
                Ok(serde_json::Value::Null)
            }
        }
    }

    /// Extract properties from DBF record
    fn extract_properties(
        &self,
        record: &shapefile::dbase::Record,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut properties = HashMap::new();

        // Iterate using into_iter() to get owned values
        for (name, value) in record.clone() {
            let json_value = self.convert_dbase_value(&value);
            properties.insert(name, json_value);
        }

        Ok(properties)
    }

    /// Convert dBase field value to JSON value
    fn convert_dbase_value(&self, value: &DbaseFieldValue) -> serde_json::Value {
        match value {
            DbaseFieldValue::Character(Some(s)) => serde_json::Value::String(s.clone()),
            DbaseFieldValue::Character(None) => serde_json::Value::Null,
            DbaseFieldValue::Numeric(Some(n)) => {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DbaseFieldValue::Numeric(None) => serde_json::Value::Null,
            DbaseFieldValue::Logical(Some(b)) => serde_json::Value::Bool(*b),
            DbaseFieldValue::Logical(None) => serde_json::Value::Null,
            DbaseFieldValue::Date(Some(date)) => {
                serde_json::Value::String(format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()))
            }
            DbaseFieldValue::Date(None) => serde_json::Value::Null,
            DbaseFieldValue::Float(Some(f)) => {
                serde_json::Number::from_f64(*f as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DbaseFieldValue::Float(None) => serde_json::Value::Null,
            DbaseFieldValue::Integer(i) => serde_json::Value::Number((*i).into()),
            DbaseFieldValue::Currency(c) => {
                serde_json::Number::from_f64(*c)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DbaseFieldValue::DateTime(dt) => {
                // Format as ISO 8601 string
                // Note: shapefile dbase::Time doesn't have hour/minute/second methods
                // We'll format just the date part
                serde_json::Value::String(format!(
                    "{:04}-{:02}-{:02}",
                    dt.date().year(),
                    dt.date().month(),
                    dt.date().day()
                ))
            }
            DbaseFieldValue::Double(d) => {
                serde_json::Number::from_f64(*d)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DbaseFieldValue::Memo(s) => serde_json::Value::String(s.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let reader = ShapefileFormatReader;
        assert_eq!(reader.supported_extensions(), &["shp"]);
    }

    #[test]
    fn test_format_name() {
        let reader = ShapefileFormatReader;
        assert_eq!(reader.format_name(), "Shapefile");
    }

    #[tokio::test]
    async fn test_validation_missing_file() {
        let reader = ShapefileFormatReader;
        let path = Path::new("/nonexistent/test.shp");
        
        let validation = reader.validate(path).await.unwrap();
        
        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_parse_epsg_from_wkt() {
        let reader = ShapefileFormatReader;
        
        // Test AUTHORITY pattern
        let wkt1 = r#"GEOGCS["WGS 84",AUTHORITY["EPSG","4326"]]"#;
        assert_eq!(reader.parse_epsg_from_wkt(wkt1), Some(4326));
        
        // Test EPSG: prefix
        let wkt2 = "EPSG:3857";
        assert_eq!(reader.parse_epsg_from_wkt(wkt2), Some(3857));
    }
}
