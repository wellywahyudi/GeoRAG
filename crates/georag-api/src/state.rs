use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::sync::Arc;

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
        }
    }
}
