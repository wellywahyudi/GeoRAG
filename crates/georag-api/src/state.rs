use georag_core::error::{GeoragError, Result};
use georag_core::models::IndexState;
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct EmbedderConfig {
    pub model: String,
    pub dimensions: usize,
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            model: "nomic-embed-text".to_string(),
            dimensions: 768,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub spatial_store: Arc<dyn SpatialStore>,
    pub vector_store: Arc<dyn VectorStore>,
    pub document_store: Arc<dyn DocumentStore>,
    pub embedder_config: EmbedderConfig,
    index_state: Arc<RwLock<Option<IndexState>>>,
}

impl AppState {
    pub fn new(
        spatial_store: Arc<dyn SpatialStore>,
        vector_store: Arc<dyn VectorStore>,
        document_store: Arc<dyn DocumentStore>,
        embedder_config: EmbedderConfig,
    ) -> Self {
        Self {
            spatial_store,
            vector_store,
            document_store,
            embedder_config,
            index_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the index state (called after build)
    pub async fn set_index_state(&self, state: IndexState) {
        let mut guard = self.index_state.write().await;
        *guard = Some(state);
    }

    /// Get the current index state
    pub async fn get_index_state(&self) -> Result<IndexState> {
        let guard = self.index_state.read().await;
        guard
            .clone()
            .ok_or_else(|| GeoragError::IndexNotBuilt("Index has not been built yet".to_string()))
    }

    /// Compute current index hash from stored data
    pub async fn compute_index_hash(&self) -> Result<String> {
        let chunk_ids = self.document_store.list_chunk_ids().await?;
        let chunks = self.document_store.get_chunks(&chunk_ids).await?;

        let mut hasher = DefaultHasher::new();

        chunks.len().hash(&mut hasher);
        for chunk in &chunks {
            chunk.id.0.hash(&mut hasher);
            chunk.content.hash(&mut hasher);
        }

        chunk_ids.len().hash(&mut hasher);
        if let Some(first_id) = chunk_ids.first() {
            if let Some(embedding) = self.vector_store.get_embedding(*first_id).await? {
                embedding.vector.len().hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }
}
