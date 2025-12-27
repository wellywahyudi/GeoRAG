//! Format abstraction layer for multi-format support
//!
//! This module provides a trait-based abstraction for reading different file formats.
//! Each format implements the `FormatReader` trait, and the `FormatRegistry` manages
//! format detection and dispatching to appropriate readers.

use async_trait::async_trait;
use std::path::Path;

use crate::error::Result;

pub mod geojson;
pub mod shapefile;
pub mod gpx;
pub mod kml;

/// Format reader trait that all format implementations must implement
#[async_trait]
pub trait FormatReader: Send + Sync {
    /// Read a dataset from the given path
    ///
    /// # Arguments
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    /// A `Dataset` containing the parsed data
    async fn read(&self, path: &Path) -> Result<FormatDataset>;

    /// Get supported file extensions (e.g., ["shp", "geojson"])
    fn supported_extensions(&self) -> &[&str];

    /// Get human-readable format name (e.g., "Shapefile", "GeoJSON")
    fn format_name(&self) -> &str;

    /// Validate file structure without full read (optional)
    ///
    /// This allows format readers to perform quick validation checks
    /// before attempting a full read operation.
    async fn validate(&self, _path: &Path) -> Result<FormatValidation> {
        // Default implementation: no validation errors or warnings
        Ok(FormatValidation::default())
    }
}

/// Result of format validation
#[derive(Debug, Clone, Default)]
pub struct FormatValidation {
    /// Validation errors that prevent reading
    pub errors: Vec<String>,
    
    /// Warnings that don't prevent reading but indicate potential issues
    pub warnings: Vec<String>,
}

impl FormatValidation {
    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Dataset representation returned by format readers
///
/// This is a temporary structure that will be converted to the core Dataset model
#[derive(Debug, Clone)]
pub struct FormatDataset {
    /// Dataset name
    pub name: String,
    
    /// Format-specific metadata
    pub format_metadata: FormatMetadata,
    
    /// CRS EPSG code
    pub crs: u32,
    
    /// Features extracted from the format
    pub features: Vec<FormatFeature>,
}

/// Format-specific metadata
#[derive(Debug, Clone)]
pub struct FormatMetadata {
    /// Format name (e.g., "Shapefile", "GeoJSON", "PDF")
    pub format_name: String,
    
    /// Optional format version
    pub format_version: Option<String>,
    
    /// Layer name for multi-layer formats (e.g., GeoPackage)
    pub layer_name: Option<String>,
    
    /// Page count for document formats
    pub page_count: Option<usize>,
    
    /// Paragraph count for document formats
    pub paragraph_count: Option<usize>,
    
    /// Extraction method used (e.g., "GDAL", "pdf-extract")
    pub extraction_method: Option<String>,
}

/// Feature extracted from a format
#[derive(Debug, Clone)]
pub struct FormatFeature {
    /// Feature identifier
    pub id: String,
    
    /// Geometry (GeoJSON-like structure), None for documents without geometry
    pub geometry: Option<serde_json::Value>,
    
    /// Feature properties
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Central registry for format readers
///
/// The registry maintains a collection of format readers and provides
/// format detection based on file extensions.
pub struct FormatRegistry {
    readers: Vec<Box<dyn FormatReader>>,
}

impl FormatRegistry {
    /// Create a new empty format registry
    pub fn new() -> Self {
        Self {
            readers: Vec::new(),
        }
    }

    /// Register a format reader
    pub fn register(&mut self, reader: Box<dyn FormatReader>) {
        self.readers.push(reader);
    }

    /// Detect format and return appropriate reader
    ///
    /// # Arguments
    /// * `path` - Path to the file
    ///
    /// # Returns
    /// Reference to the format reader that supports this file extension
    pub fn detect_format(&self, path: &Path) -> Result<&dyn FormatReader> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| crate::error::GeoragError::UnsupportedFormat {
                extension: "none".to_string(),
                supported: self.supported_formats(),
            })?;

        self.readers
            .iter()
            .find(|r| r.supported_extensions().contains(&extension))
            .map(|r| r.as_ref())
            .ok_or_else(|| crate::error::GeoragError::UnsupportedFormat {
                extension: extension.to_string(),
                supported: self.supported_formats(),
            })
    }

    /// Get list of all supported format extensions
    pub fn supported_formats(&self) -> Vec<String> {
        self.readers
            .iter()
            .flat_map(|r| r.supported_extensions())
            .map(|s| s.to_string())
            .collect()
    }

    /// Get all registered readers
    pub fn readers(&self) -> &[Box<dyn FormatReader>] {
        &self.readers
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock format reader for testing
    struct MockReader {
        extensions: Vec<&'static str>,
        name: &'static str,
    }

    #[async_trait]
    impl FormatReader for MockReader {
        async fn read(&self, _path: &Path) -> Result<FormatDataset> {
            Ok(FormatDataset {
                name: "test".to_string(),
                format_metadata: FormatMetadata {
                    format_name: self.name.to_string(),
                    format_version: None,
                    layer_name: None,
                    page_count: None,
                    paragraph_count: None,
                    extraction_method: None,
                },
                crs: 4326,
                features: vec![],
            })
        }

        fn supported_extensions(&self) -> &[&str] {
            &self.extensions
        }

        fn format_name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn test_format_registry_creation() {
        let registry = FormatRegistry::new();
        assert_eq!(registry.readers().len(), 0);
    }

    #[test]
    fn test_format_registration() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockReader {
            extensions: vec!["json", "geojson"],
            name: "GeoJSON",
        }));
        
        assert_eq!(registry.readers().len(), 1);
        assert_eq!(registry.supported_formats(), vec!["json", "geojson"]);
    }

    #[test]
    fn test_format_detection() {
        let mut registry = FormatRegistry::new();
        registry.register(Box::new(MockReader {
            extensions: vec!["json", "geojson"],
            name: "GeoJSON",
        }));
        registry.register(Box::new(MockReader {
            extensions: vec!["shp"],
            name: "Shapefile",
        }));

        let path = Path::new("test.geojson");
        let reader = registry.detect_format(path).unwrap();
        assert_eq!(reader.format_name(), "GeoJSON");

        let path = Path::new("test.shp");
        let reader = registry.detect_format(path).unwrap();
        assert_eq!(reader.format_name(), "Shapefile");
    }

    #[test]
    fn test_unsupported_format() {
        let registry = FormatRegistry::new();
        let path = Path::new("test.xyz");
        let result = registry.detect_format(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_validation_default() {
        let validation = FormatValidation::default();
        assert!(validation.is_valid());
        assert!(!validation.has_warnings());
    }

    #[test]
    fn test_format_validation_with_errors() {
        let validation = FormatValidation {
            errors: vec!["Missing file".to_string()],
            warnings: vec![],
        };
        assert!(!validation.is_valid());
        assert!(!validation.has_warnings());
    }

    #[test]
    fn test_format_validation_with_warnings() {
        let validation = FormatValidation {
            errors: vec![],
            warnings: vec!["No CRS specified".to_string()],
        };
        assert!(validation.is_valid());
        assert!(validation.has_warnings());
    }
}
