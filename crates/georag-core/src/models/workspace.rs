use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::DatasetMeta;

// Re-export from geometry module (single source of truth)
pub use super::geometry::{DistanceUnit, ValidityMode};

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Coordinate Reference System (EPSG code)
    pub crs: u32,

    /// Distance unit for spatial operations
    pub distance_unit: DistanceUnit,

    /// Geometry validity mode
    pub geometry_validity: ValidityMode,
}

/// Workspace state
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Path to the workspace directory
    pub path: PathBuf,

    /// Workspace configuration
    pub config: WorkspaceConfig,

    /// Registered datasets
    pub datasets: Vec<DatasetMeta>,

    /// Index state (if built)
    pub index_state: Option<IndexState>,
}

/// Index build state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    /// Deterministic hash of the index
    pub hash: String,

    /// When the index was built
    pub built_at: DateTime<Utc>,

    /// Embedder used for the index
    pub embedder: String,

    /// Number of chunks in the index
    pub chunk_count: usize,

    /// Embedding dimension
    pub embedding_dim: usize,
}
