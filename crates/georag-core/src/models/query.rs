//! Query-related models for spatial features and search results.

use super::geometry::{Crs, Geometry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ChunkId;

/// Unique identifier for a spatial feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub u64);

/// Spatial feature with geometry and properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique identifier
    pub id: FeatureId,

    /// Geometry (using canonical Geometry type)
    /// None for documents without inherent spatial location
    pub geometry: Option<Geometry>,

    /// Feature properties
    pub properties: HashMap<String, serde_json::Value>,

    /// CRS EPSG code
    pub crs: u32,
}

impl Feature {
    /// Create a new feature with geometry
    pub fn with_geometry(
        id: FeatureId,
        geometry: Geometry,
        properties: HashMap<String, serde_json::Value>,
        crs: u32,
    ) -> Self {
        Self { id, geometry: Some(geometry), properties, crs }
    }

    /// Create a new feature without geometry (for documents)
    pub fn without_geometry(
        id: FeatureId,
        properties: HashMap<String, serde_json::Value>,
        crs: u32,
    ) -> Self {
        Self { id, geometry: None, properties, crs }
    }

    /// Associate a geometry with this feature
    pub fn associate_geometry(&mut self, geometry: Geometry) {
        self.geometry = Some(geometry);
    }

    /// Check if this feature has geometry
    pub fn has_geometry(&self) -> bool {
        self.geometry.is_some()
    }

    /// Check if this feature should be included in spatial queries
    pub fn is_spatially_queryable(&self) -> bool {
        self.has_geometry()
    }

    /// Get the CRS as a Crs struct
    pub fn crs_struct(&self) -> Crs {
        Crs::new(self.crs, "")
    }
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
