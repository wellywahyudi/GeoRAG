//! Error types for GeoRAG

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeoragError {
    // Workspace errors
    #[error("Workspace not found at {path}")]
    WorkspaceNotFound { path: PathBuf },

    #[error("Workspace already exists at {path}")]
    WorkspaceExists { path: PathBuf },

    // Dataset errors
    #[error("Dataset not found: {name}")]
    DatasetNotFound { name: String },

    #[error("CRS mismatch: dataset has {dataset_crs}, workspace expects {workspace_crs}")]
    CrsMismatch {
        dataset_crs: String,
        workspace_crs: String,
    },

    #[error("Invalid geometry at feature {feature_id}: {reason}")]
    InvalidGeometry {
        feature_id: String,
        reason: String,
    },

    // Index errors
    #[error("Index not built. Run 'georag build' first")]
    IndexNotBuilt,

    #[error("Index is stale. Rebuild required after dataset changes")]
    IndexStale,

    // Embedder errors
    #[error("Embedder unavailable: {reason}. Try: {remediation}")]
    EmbedderUnavailable {
        reason: String,
        remediation: String,
    },

    // Configuration errors
    #[error("Missing required configuration: {key}")]
    ConfigMissing { key: String },

    #[error("Invalid configuration value for {key}: {reason}")]
    ConfigInvalid { key: String, reason: String },

    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, GeoragError>;
