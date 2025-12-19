//! Query and result models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ChunkId;

/// Unique identifier for a spatial feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub u64);

/// Spatial feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique identifier
    pub id: FeatureId,
    
    /// Geometry (stored as GeoJSON-like structure)
    pub geometry: serde_json::Value,
    
    /// Feature properties
    pub properties: HashMap<String, serde_json::Value>,
    
    /// CRS EPSG code
    pub crs: u32,
}

/// Spatial filter for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialFilter {
    /// Spatial predicate
    pub predicate: SpatialPredicate,
    
    /// Filter geometry (GeoJSON-like)
    pub geometry: Option<serde_json::Value>,
    
    /// Distance for proximity queries
    pub distance: Option<Distance>,
    
    /// CRS EPSG code
    pub crs: u32,
}

/// Spatial predicate types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpatialPredicate {
    Within,
    Intersects,
    Contains,
    BoundingBox,
}

/// Distance with unit
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Distance {
    /// Distance value
    pub value: f64,
    
    /// Distance unit
    pub unit: DistanceUnit,
}

/// Distance unit
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    Miles,
    Feet,
}

/// Scored search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    /// Chunk ID
    pub chunk_id: ChunkId,
    
    /// Similarity score
    pub score: f32,
    
    /// Optional spatial score
    pub spatial_score: Option<f32>,
}
