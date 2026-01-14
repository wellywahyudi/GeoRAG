pub mod dataset;
pub mod document;
pub mod geometry;
pub mod query;
pub mod workspace;

pub use dataset::{Dataset, DatasetId, DatasetMeta};
pub use document::{ChunkId, ChunkMetadata, ChunkSource, Embedding, SpatialMetadata, TextChunk};
pub use geometry::{
    Crs, Distance, DistanceUnit, Geometry, GeometryType, SpatialFilter, SpatialPredicate,
    ValidityMode,
};
pub use query::{Feature, FeatureId, ScoredResult};
pub use workspace::{IndexState, Workspace, WorkspaceConfig};
