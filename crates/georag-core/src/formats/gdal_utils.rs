//! GDAL utility functions and error handling helpers
//!
//! This module provides common utilities for working with GDAL,
//! including error conversion and resource management.

use crate::error::GeoragError;
use gdal::errors::GdalError;
use gdal::vector::LayerAccess;
use std::path::Path;

/// Convert GDAL errors to GeoRAG errors with context
pub fn convert_gdal_error(err: GdalError, context: &str) -> GeoragError {
    GeoragError::FormatError {
        format: "GDAL".to_string(),
        message: format!("{}: {}", context, err),
    }
}

/// Verify that a file exists and is readable
pub fn verify_file_exists(path: &Path) -> Result<(), GeoragError> {
    if !path.exists() {
        return Err(GeoragError::FileNotFound {
            path: path.to_path_buf(),
        });
    }
    
    if !path.is_file() {
        return Err(GeoragError::InvalidPath {
            path: path.to_path_buf(),
            reason: "Path is not a file".to_string(),
        });
    }
    
    Ok(())
}

/// Check if GDAL can open a dataset without fully loading it
pub fn validate_gdal_dataset(path: &Path) -> Result<(), GeoragError> {
    use gdal::Dataset;
    
    verify_file_exists(path)?;
    
    Dataset::open(path)
        .map_err(|e| convert_gdal_error(e, "Failed to validate dataset"))?;
    
    Ok(())
}

/// Get the number of layers in a GDAL dataset
pub fn get_layer_count(path: &Path) -> Result<usize, GeoragError> {
    use gdal::Dataset;
    
    let dataset = Dataset::open(path)
        .map_err(|e| convert_gdal_error(e, "Failed to open dataset"))?;
    
    Ok(dataset.layer_count() as usize)
}

/// Get layer names from a GDAL dataset
pub fn get_layer_names(path: &Path) -> Result<Vec<String>, GeoragError> {
    use gdal::Dataset;
    
    let dataset = Dataset::open(path)
        .map_err(|e| convert_gdal_error(e, "Failed to open dataset"))?;
    
    let mut names = Vec::new();
    for i in 0..dataset.layer_count() {
        let layer = dataset.layer(i)
            .map_err(|e| convert_gdal_error(e, &format!("Failed to access layer {}", i)))?;
        names.push(layer.name());
    }
    
    Ok(names)
}

/// Extract CRS information from a GDAL spatial reference
pub fn extract_crs_from_spatial_ref(
    spatial_ref: &gdal::spatial_ref::SpatialRef,
) -> Result<String, GeoragError> {
    // Try to get EPSG code first
    if let Ok(code) = spatial_ref.auth_code() {
        return Ok(format!("EPSG:{}", code));
    }
    
    // Fall back to WKT representation
    spatial_ref
        .to_wkt()
        .map_err(|e| convert_gdal_error(e, "Failed to extract CRS as WKT"))
}

/// Check if a path has a specific extension
pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

/// Get the base path for a Shapefile (without extension)
/// This is useful for finding component files (.shp, .shx, .dbf, .prj)
pub fn get_shapefile_base(path: &Path) -> Result<std::path::PathBuf, GeoragError> {
    if !has_extension(path, "shp") {
        return Err(GeoragError::InvalidPath {
            path: path.to_path_buf(),
            reason: "Not a Shapefile (.shp)".to_string(),
        });
    }
    
    Ok(path.with_extension(""))
}

/// Verify that all required Shapefile component files exist
pub fn verify_shapefile_components(path: &Path) -> Result<(), GeoragError> {
    let base = get_shapefile_base(path)?;
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

/// Check if a Shapefile has a .prj file (projection information)
pub fn has_shapefile_projection(path: &Path) -> Result<bool, GeoragError> {
    let base = get_shapefile_base(path)?;
    let prj_path = base.with_extension("prj");
    Ok(prj_path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_has_extension() {
        let path = PathBuf::from("test.shp");
        assert!(has_extension(&path, "shp"));
        assert!(has_extension(&path, "SHP"));
        assert!(!has_extension(&path, "gpkg"));
    }
    
    #[test]
    fn test_get_shapefile_base() {
        let path = PathBuf::from("/data/cities.shp");
        let base = get_shapefile_base(&path).unwrap();
        assert_eq!(base, PathBuf::from("/data/cities"));
    }
    
    #[test]
    fn test_get_shapefile_base_invalid() {
        let path = PathBuf::from("/data/cities.gpkg");
        assert!(get_shapefile_base(&path).is_err());
    }
}
