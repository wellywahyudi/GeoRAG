//! Core domain models

pub mod workspace;
pub mod dataset;
pub mod document;
pub mod query;

pub use workspace::{IndexState, Workspace, WorkspaceConfig};
pub use dataset::{Dataset, DatasetId, DatasetMeta};
pub use document::{ChunkId, ChunkMetadata, ChunkSource, Embedding, SpatialMetadata, TextChunk};
pub use query::{Feature, FeatureId, ScoredResult, SpatialFilter};
