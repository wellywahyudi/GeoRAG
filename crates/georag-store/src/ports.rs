use async_trait::async_trait;
use georag_core::error::Result;
use georag_core::models::{
    ChunkId, Dataset, DatasetId, DatasetMeta, Embedding, Feature, FeatureId, ScoredResult,
    SpatialFilter, TextChunk,
};

/// Port for spatial data storage operations
#[async_trait]
pub trait SpatialStore: Send + Sync {
    /// Store a new dataset
    async fn store_dataset(&self, dataset: &Dataset) -> Result<DatasetId>;

    /// Retrieve a dataset by ID
    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>>;

    /// List all dataset metadata
    async fn list_datasets(&self) -> Result<Vec<DatasetMeta>>;

    /// Delete a dataset
    async fn delete_dataset(&self, id: DatasetId) -> Result<()>;

    /// Store spatial features
    async fn store_features(&self, features: &[Feature]) -> Result<()>;

    /// Query features using spatial filter
    async fn spatial_query(&self, filter: &SpatialFilter) -> Result<Vec<Feature>>;

    /// Get a specific feature by ID
    async fn get_feature(&self, id: FeatureId) -> Result<Option<Feature>>;
}

/// Port for vector storage and similarity search
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store embeddings
    async fn store_embeddings(&self, embeddings: &[Embedding]) -> Result<()>;

    /// Perform similarity search
    /// Returns the top k most similar embeddings to the query vector
    /// If threshold is provided, only returns results with similarity >= threshold
    async fn similarity_search(
        &self,
        query: &[f32],
        k: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<ScoredResult>>;

    /// Get embedding by chunk ID
    async fn get_embedding(&self, chunk_id: ChunkId) -> Result<Option<Embedding>>;

    /// Delete embeddings by chunk IDs
    async fn delete_embeddings(&self, chunk_ids: &[ChunkId]) -> Result<()>;

    /// Get the dimensionality of stored vectors
    async fn dimensions(&self) -> Result<usize>;
}

/// Port for document chunk storage
#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// Store text chunks
    async fn store_chunks(&self, chunks: &[TextChunk]) -> Result<()>;

    /// Retrieve chunks by IDs
    async fn get_chunks(&self, ids: &[ChunkId]) -> Result<Vec<TextChunk>>;

    /// Get a single chunk by ID
    async fn get_chunk(&self, id: ChunkId) -> Result<Option<TextChunk>>;

    /// Delete chunks by IDs
    async fn delete_chunks(&self, ids: &[ChunkId]) -> Result<()>;

    /// List all chunk IDs
    async fn list_chunk_ids(&self) -> Result<Vec<ChunkId>>;
}
