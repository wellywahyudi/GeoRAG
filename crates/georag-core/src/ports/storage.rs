use crate::error::Result;
use crate::models::{
    ChunkId, Dataset, DatasetId, DatasetMeta, Embedding, Feature, ScoredResult, SpatialFilter,
    TextChunk,
};

/// Port for spatial data storage operations
pub trait SpatialStore {
    /// Store a dataset in the spatial store
    fn store_dataset(&self, dataset: &Dataset) -> Result<DatasetId>;

    /// Retrieve a dataset by ID
    fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>>;

    /// List all datasets with metadata
    fn list_datasets(&self) -> Result<Vec<DatasetMeta>>;

    /// Execute a spatial query with filters
    fn spatial_query(&self, filter: &SpatialFilter) -> Result<Vec<Feature>>;
}

/// Port for vector embedding storage and similarity search
pub trait VectorStore {
    /// Store embeddings in the vector store
    fn store_embeddings(&self, embeddings: &[Embedding]) -> Result<()>;

    /// Perform similarity search for k nearest neighbors
    fn similarity_search(&self, query: &[f32], k: usize) -> Result<Vec<ScoredResult>>;
}

/// Port for document chunk storage
pub trait DocumentStore {
    /// Store text chunks
    fn store_chunks(&self, chunks: &[TextChunk]) -> Result<()>;

    /// Retrieve chunks by IDs
    fn get_chunks(&self, ids: &[ChunkId]) -> Result<Vec<TextChunk>>;
}
