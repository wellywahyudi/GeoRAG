//! Integration tests for GPX format reader

use georag_core::formats::{FormatReader, gpx::GpxReader};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_gpx_complete_workflow() {
    let reader = GpxReader;
    
    // Create a temporary GPX file with all feature types
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("complete.gpx");
    
    let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <metadata>
    <name>Complete Test</name>
  </metadata>
  
  <wpt lat="47.644548" lon="-122.326897">
    <ele>4.46</ele>
    <name>Waypoint 1</name>
  </wpt>
  
  <trk>
    <name>Track 1</name>
    <trkseg>
      <trkpt lat="47.644548" lon="-122.326897">
        <ele>4.46</ele>
      </trkpt>
      <trkpt lat="47.644649" lon="-122.326998">
        <ele>4.50</ele>
      </trkpt>
    </trkseg>
  </trk>
  
  <rte>
    <name>Route 1</name>
    <rtept lat="47.644548" lon="-122.326897">
      <ele>4.46</ele>
    </rtept>
    <rtept lat="47.645000" lon="-122.327500">
      <ele>5.00</ele>
    </rtept>
  </rte>
</gpx>"#;
    
    fs::write(&file_path, gpx_content).unwrap();
    
    // Read the GPX file
    let result = reader.read(&file_path).await.unwrap();
    
    // Verify basic metadata
    assert_eq!(result.name, "complete");
    assert_eq!(result.format_metadata.format_name, "GPX");
    assert_eq!(result.crs, 4326); // GPX always uses WGS84
    
    // Verify we have all three feature types
    assert_eq!(result.features.len(), 3); // 1 waypoint + 1 track + 1 route
    
    // Verify waypoint
    let waypoint = result.features.iter().find(|f| f.id.starts_with("waypoint")).unwrap();
    assert!(waypoint.geometry.is_some());
    assert_eq!(waypoint.geometry.as_ref().unwrap()["type"], "Point");
    assert_eq!(waypoint.properties.get("type").unwrap(), "waypoint");
    assert_eq!(waypoint.properties.get("name").unwrap(), "Waypoint 1");
    
    // Verify track
    let track = result.features.iter().find(|f| f.id.starts_with("track")).unwrap();
    assert!(track.geometry.is_some());
    assert_eq!(track.geometry.as_ref().unwrap()["type"], "LineString");
    assert_eq!(track.properties.get("type").unwrap(), "track");
    assert_eq!(track.properties.get("name").unwrap(), "Track 1");
    
    // Verify route
    let route = result.features.iter().find(|f| f.id.starts_with("route")).unwrap();
    assert!(route.geometry.is_some());
    assert_eq!(route.geometry.as_ref().unwrap()["type"], "LineString");
    assert_eq!(route.properties.get("type").unwrap(), "route");
    assert_eq!(route.properties.get("name").unwrap(), "Route 1");
}

#[tokio::test]
async fn test_gpx_elevation_handling() {
    let reader = GpxReader;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("elevation.gpx");
    
    let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <wpt lat="47.644548" lon="-122.326897">
    <ele>100.5</ele>
    <name>High Point</name>
  </wpt>
  
  <trk>
    <name>Elevation Track</name>
    <trkseg>
      <trkpt lat="47.644548" lon="-122.326897">
        <ele>100.0</ele>
      </trkpt>
      <trkpt lat="47.644649" lon="-122.326998">
        <ele>150.0</ele>
      </trkpt>
      <trkpt lat="47.644750" lon="-122.327099">
        <ele>200.0</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;
    
    fs::write(&file_path, gpx_content).unwrap();
    
    let result = reader.read(&file_path).await.unwrap();
    
    // Check waypoint has 3D coordinates
    let waypoint = &result.features[0];
    let coords = &waypoint.geometry.as_ref().unwrap()["coordinates"];
    assert_eq!(coords.as_array().unwrap().len(), 3); // [lon, lat, ele]
    assert_eq!(coords[2], 100.5);
    
    // Check track has 3D coordinates
    let track = &result.features[1];
    let track_coords = &track.geometry.as_ref().unwrap()["coordinates"];
    assert_eq!(track_coords[0].as_array().unwrap().len(), 3);
    assert_eq!(track_coords[0][2], 100.0);
    assert_eq!(track_coords[1][2], 150.0);
    assert_eq!(track_coords[2][2], 200.0);
}

#[tokio::test]
async fn test_gpx_track_segments() {
    let reader = GpxReader;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("segments.gpx");
    
    let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <trk>
    <name>Multi-Segment Track</name>
    <trkseg>
      <trkpt lat="47.644548" lon="-122.326897">
        <ele>4.46</ele>
      </trkpt>
      <trkpt lat="47.644649" lon="-122.326998">
        <ele>4.50</ele>
      </trkpt>
    </trkseg>
    <trkseg>
      <trkpt lat="47.645000" lon="-122.327500">
        <ele>5.00</ele>
      </trkpt>
      <trkpt lat="47.645100" lon="-122.327600">
        <ele>5.10</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;
    
    fs::write(&file_path, gpx_content).unwrap();
    
    let result = reader.read(&file_path).await.unwrap();
    
    // Should have 2 features (one per segment)
    assert_eq!(result.features.len(), 2);
    
    // Verify both are tracks with correct segment numbers
    assert_eq!(result.features[0].id, "track_0_0");
    assert_eq!(result.features[0].properties.get("segment").unwrap(), 0);
    
    assert_eq!(result.features[1].id, "track_0_1");
    assert_eq!(result.features[1].properties.get("segment").unwrap(), 1);
}

#[tokio::test]
async fn test_gpx_metadata_extraction() {
    let reader = GpxReader;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("metadata.gpx");
    
    let gpx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="GeoRAG">
  <wpt lat="47.644548" lon="-122.326897">
    <name>Test Point</name>
    <desc>A test description</desc>
  </wpt>
</gpx>"#;
    
    fs::write(&file_path, gpx_content).unwrap();
    
    let result = reader.read(&file_path).await.unwrap();
    
    // Verify format metadata
    assert_eq!(result.format_metadata.format_name, "GPX");
    assert!(result.format_metadata.format_version.is_some());
    assert_eq!(result.format_metadata.extraction_method, Some("gpx-rs".to_string()));
    
    // Verify feature metadata
    let waypoint = &result.features[0];
    assert_eq!(waypoint.properties.get("name").unwrap(), "Test Point");
    assert_eq!(waypoint.properties.get("description").unwrap(), "A test description");
}
