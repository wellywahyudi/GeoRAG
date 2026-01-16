use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::geometry::GeometryType;

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

    /// Spatial association metadata for documents
    pub spatial_association: Option<SpatialAssociation>,
}

/// Spatial association metadata for documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialAssociation {
    /// Source of the spatial association (e.g., "manual", "file", "geocoding")
    pub source: String,

    /// Path to the geometry file if association came from a file
    pub geometry_file: Option<PathBuf>,

    /// Timestamp when association was created
    pub associated_at: DateTime<Utc>,

    /// Description of the association
    pub description: Option<String>,
}
