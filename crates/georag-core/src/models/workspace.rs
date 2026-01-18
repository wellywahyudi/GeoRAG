use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::DatasetMeta;

// Re-export from geometry module (single source of truth)
pub use super::geometry::{DistanceUnit, ValidityMode};

/// Unique identifier for a workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Workspace metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    pub id: WorkspaceId,
    pub name: String,
    pub crs: u32,
    pub distance_unit: DistanceUnit,
    pub geometry_validity: ValidityMode,
    pub created_at: DateTime<Utc>,
}

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
