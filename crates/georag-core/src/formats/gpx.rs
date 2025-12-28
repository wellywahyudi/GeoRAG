use async_trait::async_trait;
use gpx::{read, Gpx};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::error::{GeoragError, Result};
use crate::formats::validation::FormatValidator;
use crate::formats::{
    FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation,
};

/// GPX format reader
pub struct GpxReader;

#[async_trait]
impl FormatReader for GpxReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        self.read_internal(path, None).await
    }

    async fn read_with_options(
        &self,
        path: &Path,
        options: &crate::formats::FormatOptions,
    ) -> Result<FormatDataset> {
        let track_type = options.get("track_type").map(|s| s.as_str());
        self.read_internal(path, track_type).await
    }

    fn supported_extensions(&self) -> &[&str] {
        &["gpx"]
    }

    fn format_name(&self) -> &str {
        "GPX"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        // Basic file validation
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Validate XML structure
        let xml_validation = FormatValidator::validate_xml_structure(path);

        // If XML is valid, try to parse as GPX
        if xml_validation.is_valid() {
            match File::open(path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    if let Err(e) = read(reader) {
                        validation.errors.push(format!("Invalid GPX: {}", e));
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

impl GpxReader {
    /// Internal read method that supports track type filtering
    async fn read_internal(&self, path: &Path, track_type: Option<&str>) -> Result<FormatDataset> {
        // Open and parse the GPX file
        let file = File::open(path).map_err(|e| GeoragError::FormatError {
            format: "GPX".to_string(),
            message: format!("Failed to open GPX file: {}", e),
        })?;

        let reader = BufReader::new(file);
        let gpx: Gpx = read(reader).map_err(|e| GeoragError::FormatValidation {
            format: "GPX".to_string(),
            reason: format!("Failed to parse GPX: {}", e),
        })?;

        // Extract features based on track type filter
        let mut features = Vec::new();

        match track_type {
            Some("waypoints") => {
                features.extend(self.extract_waypoints(&gpx)?);
            }
            Some("tracks") => {
                features.extend(self.extract_tracks(&gpx)?);
            }
            Some("routes") => {
                features.extend(self.extract_routes(&gpx)?);
            }
            Some("all") | None => {
                // Extract all types (default behavior)
                features.extend(self.extract_waypoints(&gpx)?);
                features.extend(self.extract_tracks(&gpx)?);
                features.extend(self.extract_routes(&gpx)?);
            }
            Some(other) => {
                return Err(GeoragError::FormatError {
                    format: "GPX".to_string(),
                    message: format!(
                        "Invalid track type '{}'. Valid options: waypoints, tracks, routes, all",
                        other
                    ),
                });
            }
        }

        // Get dataset name from filename
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed").to_string();

        // Extract metadata
        let metadata = self.extract_metadata(&gpx);

        Ok(FormatDataset {
            name,
            format_metadata: metadata,
            crs: 4326, // GPX always uses WGS84 (EPSG:4326) per specification
            features,
        })
    }
    /// Extract waypoints from GPX as Point features
    fn extract_waypoints(&self, gpx: &Gpx) -> Result<Vec<FormatFeature>> {
        let mut features = Vec::new();

        for (idx, waypoint) in gpx.waypoints.iter().enumerate() {
            let mut properties = HashMap::new();

            // Add waypoint metadata
            properties.insert("type".to_string(), serde_json::json!("waypoint"));

            if let Some(name) = &waypoint.name {
                properties.insert("name".to_string(), serde_json::json!(name));
            }

            if let Some(desc) = &waypoint.description {
                properties.insert("description".to_string(), serde_json::json!(desc));
            }

            if let Some(time) = waypoint.time {
                if let Ok(time_str) = time.format() {
                    properties.insert("time".to_string(), serde_json::json!(time_str));
                }
            }

            if let Some(elevation) = waypoint.elevation {
                properties.insert("elevation".to_string(), serde_json::json!(elevation));
            }

            // Create Point geometry with optional elevation
            let geometry = if let Some(elevation) = waypoint.elevation {
                serde_json::json!({
                    "type": "Point",
                    "coordinates": [waypoint.point().x(), waypoint.point().y(), elevation]
                })
            } else {
                serde_json::json!({
                    "type": "Point",
                    "coordinates": [waypoint.point().x(), waypoint.point().y()]
                })
            };

            features.push(FormatFeature {
                id: format!("waypoint_{}", idx),
                geometry: Some(geometry),
                properties,
            });
        }

        Ok(features)
    }

    /// Extract tracks from GPX as LineString features
    fn extract_tracks(&self, gpx: &Gpx) -> Result<Vec<FormatFeature>> {
        let mut features = Vec::new();

        for (track_idx, track) in gpx.tracks.iter().enumerate() {
            // Each track can have multiple segments
            for (seg_idx, segment) in track.segments.iter().enumerate() {
                let mut properties = HashMap::new();

                // Add track metadata
                properties.insert("type".to_string(), serde_json::json!("track"));

                if let Some(name) = &track.name {
                    properties.insert("name".to_string(), serde_json::json!(name));
                }

                if let Some(desc) = &track.description {
                    properties.insert("description".to_string(), serde_json::json!(desc));
                }

                properties.insert("segment".to_string(), serde_json::json!(seg_idx));

                // Extract track points with elevation if available
                let has_elevation = segment.points.iter().any(|p| p.elevation.is_some());

                let coordinates: Vec<serde_json::Value> = if has_elevation {
                    segment
                        .points
                        .iter()
                        .map(|point| {
                            let elevation = point.elevation.unwrap_or(0.0);
                            serde_json::json!([point.point().x(), point.point().y(), elevation])
                        })
                        .collect()
                } else {
                    segment
                        .points
                        .iter()
                        .map(|point| serde_json::json!([point.point().x(), point.point().y()]))
                        .collect()
                };

                // Create LineString geometry
                let geometry = serde_json::json!({
                    "type": "LineString",
                    "coordinates": coordinates
                });

                features.push(FormatFeature {
                    id: format!("track_{}_{}", track_idx, seg_idx),
                    geometry: Some(geometry),
                    properties,
                });
            }
        }

        Ok(features)
    }

    /// Extract routes from GPX as LineString features
    fn extract_routes(&self, gpx: &Gpx) -> Result<Vec<FormatFeature>> {
        let mut features = Vec::new();

        for (idx, route) in gpx.routes.iter().enumerate() {
            let mut properties = HashMap::new();

            // Add route metadata
            properties.insert("type".to_string(), serde_json::json!("route"));

            if let Some(name) = &route.name {
                properties.insert("name".to_string(), serde_json::json!(name));
            }

            if let Some(desc) = &route.description {
                properties.insert("description".to_string(), serde_json::json!(desc));
            }

            // Extract route points with elevation if available
            let has_elevation = route.points.iter().any(|p| p.elevation.is_some());

            let coordinates: Vec<serde_json::Value> = if has_elevation {
                route
                    .points
                    .iter()
                    .map(|point| {
                        let elevation = point.elevation.unwrap_or(0.0);
                        serde_json::json!([point.point().x(), point.point().y(), elevation])
                    })
                    .collect()
            } else {
                route
                    .points
                    .iter()
                    .map(|point| serde_json::json!([point.point().x(), point.point().y()]))
                    .collect()
            };

            // Create LineString geometry
            let geometry = serde_json::json!({
                "type": "LineString",
                "coordinates": coordinates
            });

            features.push(FormatFeature {
                id: format!("route_{}", idx),
                geometry: Some(geometry),
                properties,
            });
        }

        Ok(features)
    }

    /// Extract GPX metadata
    fn extract_metadata(&self, gpx: &Gpx) -> FormatMetadata {
        FormatMetadata {
            format_name: "GPX".to_string(),
            format_version: gpx.version.to_string().into(),
            layer_name: None,
            page_count: None,
            paragraph_count: None,
            extraction_method: Some("gpx-rs".to_string()),
            spatial_association: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_gpx_reader_waypoints() {
        let reader = GpxReader;

        // Create a temporary GPX file with waypoints
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.gpx");

        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <wpt lat="47.644548" lon="-122.326897">
    <ele>4.46</ele>
    <name>Test Waypoint</name>
    <desc>A test waypoint</desc>
  </wpt>
</gpx>"#;

        fs::write(&file_path, gpx_content).unwrap();

        let result = reader.read(&file_path).await.unwrap();

        assert_eq!(result.name, "test");
        assert_eq!(result.format_metadata.format_name, "GPX");
        assert_eq!(result.crs, 4326);
        assert_eq!(result.features.len(), 1);
        assert_eq!(result.features[0].id, "waypoint_0");
        assert!(result.features[0].geometry.is_some());
    }

    #[tokio::test]
    async fn test_gpx_reader_tracks() {
        let reader = GpxReader;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.gpx");

        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <trk>
    <name>Test Track</name>
    <trkseg>
      <trkpt lat="47.644548" lon="-122.326897">
        <ele>4.46</ele>
      </trkpt>
      <trkpt lat="47.644549" lon="-122.326898">
        <ele>4.47</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        fs::write(&file_path, gpx_content).unwrap();

        let result = reader.read(&file_path).await.unwrap();

        assert_eq!(result.features.len(), 1);
        assert_eq!(result.features[0].id, "track_0_0");

        // Verify it's a LineString
        let geometry = result.features[0].geometry.as_ref().unwrap();
        assert_eq!(geometry["type"], "LineString");
    }

    #[tokio::test]
    async fn test_gpx_reader_routes() {
        let reader = GpxReader;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.gpx");

        let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <rte>
    <name>Test Route</name>
    <rtept lat="47.644548" lon="-122.326897">
      <ele>4.46</ele>
    </rtept>
    <rtept lat="47.644549" lon="-122.326898">
      <ele>4.47</ele>
    </rtept>
  </rte>
</gpx>"#;

        fs::write(&file_path, gpx_content).unwrap();

        let result = reader.read(&file_path).await.unwrap();

        assert_eq!(result.features.len(), 1);
        assert_eq!(result.features[0].id, "route_0");
    }

    #[tokio::test]
    async fn test_gpx_reader_validation() {
        let reader = GpxReader;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("invalid.gpx");

        // Write invalid XML
        fs::write(&file_path, "not valid xml").unwrap();

        let validation = reader.validate(&file_path).await.unwrap();

        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_supported_extensions() {
        let reader = GpxReader;
        assert_eq!(reader.supported_extensions(), &["gpx"]);
    }

    #[test]
    fn test_format_name() {
        let reader = GpxReader;
        assert_eq!(reader.format_name(), "GPX");
    }
}
