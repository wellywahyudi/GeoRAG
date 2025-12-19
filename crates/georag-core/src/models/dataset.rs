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
