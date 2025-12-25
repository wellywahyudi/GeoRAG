//! Dataset models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a dataset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetId(pub u64);

/// Dataset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMeta {
    /// Unique identifier
    pub id: DatasetId,
    
    /// Dataset name
    pub name: String,
    
    /// Geometry type
    pub geometry_type: GeometryType,
    
    /// Number of features
    pub feature_count: usize,
    
    /// CRS EPSG code
    pub crs: u32,
    
    /// When the dataset was added
    pub added_at: DateTime<Utc>,
}

/// Full dataset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    /// Unique identifier
    pub id: DatasetId,
    
    /// Dataset name
    pub name: String,
    
    /// Path to the dataset file
    pub path: PathBuf,
    
    /// Geometry type
    pub geometry_type: GeometryType,
    
    /// Number of features
    pub feature_count: usize,
    
    /// CRS EPSG code
    pub crs: u32,
    
    /// Format-specific metadata
    pub format: FormatMetadata,
    
    /// When the dataset was added
    pub added_at: DateTime<Utc>,
}

/// Geometry type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    GeometryCollection,
}

/// Format-specific metadata for datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatMetadata {
    /// Format name (e.g., "GeoJSON", "Shapefile", "PDF")
    pub format_name: String,
    
    /// Optional format version
    pub format_version: Option<String>,
    
    /// Optional layer name (for multi-layer formats like GeoPackage)
    pub layer_name: Option<String>,
    
    /// Optional page count (for document formats like PDF)
    pub page_count: Option<usize>,
    
    /// Optional paragraph count (for document formats like DOCX)
    pub paragraph_count: Option<usize>,
    
    /// Optional extraction method (e.g., "GDAL", "pdf-extract", "docx-rs")
    pub extraction_method: Option<String>,
}
