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

    // Format errors
    #[error("Unsupported format: .{extension}. Supported formats: {}", supported.join(", "))]
    UnsupportedFormat {
        extension: String,
        supported: Vec<String>,
    },

    #[error("Format error for {format}: {message}")]
    FormatError {
        format: String,
        message: String,
    },

    #[error("Format validation failed for {format}: {reason}")]
    FormatValidation {
        format: String,
        reason: String,
    },

    #[error("Document extraction failed for {format}: {reason}")]
    DocumentExtraction {
        format: String,
        reason: String,
    },

    #[error("Layer '{layer}' not found. Available layers: {}", available.join(", "))]
    LayerNotFound {
        layer: String,
        available: Vec<String>,
    },

    #[error("CRS extraction failed for {format}: {reason}")]
    CrsExtraction {
        format: String,
        reason: String,
    },

    #[error("Missing required components for {format}: {}", missing.join(", "))]
    ComponentMissing {
        format: String,
        missing: Vec<String>,
    },

    #[error("File not found: {path}")]
    FileNotFound {
        path: PathBuf,
    },

    #[error("Invalid path {path}: {reason}")]
    InvalidPath {
        path: PathBuf,
        reason: String,
    },
}

pub type Result<T> = std::result::Result<T, GeoragError>;
