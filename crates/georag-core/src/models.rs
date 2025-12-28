pub mod dataset;
pub mod document;
pub mod query;
pub mod workspace;

pub use dataset::{Dataset, DatasetId, DatasetMeta};
pub use document::{ChunkId, ChunkMetadata, ChunkSource, Embedding, SpatialMetadata, TextChunk};
pub use query::{Feature, FeatureId, ScoredResult, SpatialFilter};
pub use workspace::{IndexState, Workspace, WorkspaceConfig};
