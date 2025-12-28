use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::FeatureId;

/// Unique identifier for a text chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub u64);

/// Text chunk extracted from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique identifier
    pub id: ChunkId,

    /// Text content
    pub content: String,

    /// Source information
    pub source: ChunkSource,

    /// Optional spatial reference
    pub spatial_ref: Option<FeatureId>,

    /// Additional metadata
    pub metadata: ChunkMetadata,
}

/// Source of a text chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSource {
    /// Source document path
    pub document_path: String,

    /// Page number (for PDFs)
    pub page: Option<usize>,

    /// Character offset in source
    pub offset: usize,
}

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Chunk size in characters
    pub size: usize,

    /// Additional properties
    pub properties: HashMap<String, String>,
}

/// Embedding vector with spatial metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Associated chunk ID
    pub chunk_id: ChunkId,

    /// Embedding vector
    pub vector: Vec<f32>,

    /// Optional spatial metadata
    pub spatial_metadata: Option<SpatialMetadata>,
}

/// Spatial metadata attached to embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMetadata {
    /// Associated feature ID
    pub feature_id: FeatureId,

    /// CRS EPSG code
    pub crs: u32,

    /// Bounding box [min_x, min_y, max_x, max_y]
    pub bbox: Option<[f64; 4]>,
}
