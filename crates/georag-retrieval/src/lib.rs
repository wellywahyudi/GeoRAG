//! GeoRAG Retrieval - Search pipelines and ranking
//!
//! This crate implements the retrieval use cases, orchestrating spatial filtering
//! and semantic search operations.

pub mod index;
pub mod models;
pub mod pipeline;

pub use index::{IndexBuildResult, IndexBuilder};
pub use models::{
    QueryExplanation, QueryPlan, QueryResult, RankingDetail, SemanticPhaseExplanation,
    SourceReference, SpatialPhaseExplanation,
};
pub use pipeline::RetrievalPipeline;
