use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use georag_core::error::GeoragError;
use georag_core::models::{IndexState, WorkspaceId};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore, WorkspaceStore};
use tokio::sync::RwLock;

use crate::config::EmbedderConfig;
use crate::error::ApiError;

/// Rebuild status for a workspace
#[derive(Debug, Clone)]
pub enum RebuildStatus {
    InProgress,
    Completed,
    Failed(String),
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub spatial_store: Arc<dyn SpatialStore>,
    pub vector_store: Arc<dyn VectorStore>,
    pub document_store: Arc<dyn DocumentStore>,
    pub workspace_store: Arc<dyn WorkspaceStore>,
    pub embedder_config: EmbedderConfig,
    index_state: Arc<RwLock<Option<IndexState>>>,
    workspace_index_states: Arc<RwLock<HashMap<WorkspaceId, IndexState>>>,
    rebuild_status: Arc<RwLock<HashMap<WorkspaceId, RebuildStatus>>>,
}

impl AppState {
    pub fn new(
        spatial_store: Arc<dyn SpatialStore>,
        vector_store: Arc<dyn VectorStore>,
        document_store: Arc<dyn DocumentStore>,
        workspace_store: Arc<dyn WorkspaceStore>,
        embedder_config: EmbedderConfig,
    ) -> Self {
        Self {
            spatial_store,
            vector_store,
            document_store,
            workspace_store,
            embedder_config,
            index_state: Arc::new(RwLock::new(None)),
            workspace_index_states: Arc::new(RwLock::new(HashMap::new())),
            rebuild_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the index state (called after build)
    pub async fn set_index_state(&self, state: IndexState) {
        let mut guard = self.index_state.write().await;
        *guard = Some(state);
    }

    /// Get the current index state
    pub async fn get_index_state(&self) -> Result<IndexState, ApiError> {
        let guard = self.index_state.read().await;
        guard.clone().ok_or_else(|| ApiError::not_found("Index has not been built yet"))
    }

    /// Compute current index hash from stored data
    pub async fn compute_index_hash(&self) -> Result<String, ApiError> {
        let chunk_ids =
            self.document_store.list_chunk_ids().await.map_err(|e| {
                ApiError::internal("Failed to list chunks").with_details(e.to_string())
            })?;

        let chunks =
            self.document_store.get_chunks(&chunk_ids).await.map_err(|e| {
                ApiError::internal("Failed to get chunks").with_details(e.to_string())
            })?;

        let mut hasher = DefaultHasher::new();

        chunks.len().hash(&mut hasher);
        for chunk in &chunks {
            chunk.id.0.hash(&mut hasher);
            chunk.content.hash(&mut hasher);
        }

        chunk_ids.len().hash(&mut hasher);
        if let Some(first_id) = chunk_ids.first() {
            if let Some(embedding) =
                self.vector_store.get_embedding(*first_id).await.map_err(|e| {
                    ApiError::internal("Failed to get embedding").with_details(e.to_string())
                })?
            {
                embedding.vector.len().hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Check if a workspace is currently rebuilding
    pub async fn is_rebuilding(&self, workspace_id: WorkspaceId) -> bool {
        let guard = self.rebuild_status.read().await;
        matches!(guard.get(&workspace_id), Some(RebuildStatus::InProgress))
    }

    /// Mark a workspace as rebuilding
    pub async fn start_rebuild(&self, workspace_id: WorkspaceId) {
        let mut guard = self.rebuild_status.write().await;
        guard.insert(workspace_id, RebuildStatus::InProgress);
    }

    /// Mark a workspace rebuild as finished
    pub async fn finish_rebuild(&self, workspace_id: WorkspaceId) {
        let mut guard = self.rebuild_status.write().await;
        if matches!(guard.get(&workspace_id), Some(RebuildStatus::InProgress)) {
            guard.insert(workspace_id, RebuildStatus::Completed);
        }
    }

    /// Mark a workspace rebuild as failed
    pub async fn set_rebuild_error(&self, workspace_id: WorkspaceId, error: String) {
        let mut guard = self.rebuild_status.write().await;
        guard.insert(workspace_id, RebuildStatus::Failed(error));
    }

    /// Get index state for a specific workspace
    pub async fn get_workspace_index_state(&self, workspace_id: WorkspaceId) -> Option<IndexState> {
        let guard = self.workspace_index_states.read().await;
        guard.get(&workspace_id).cloned()
    }

    /// Set index state for a specific workspace
    pub async fn set_workspace_index_state(&self, workspace_id: WorkspaceId, state: IndexState) {
        let mut guard = self.workspace_index_states.write().await;
        guard.insert(workspace_id, state);
    }

    /// Rebuild index for a workspace using the shared IndexBuilder
    pub async fn rebuild_index_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<(), GeoragError> {
        use georag_geo::models::Crs;
        use georag_llm::ollama::OllamaEmbedder;
        use georag_retrieval::IndexBuilder;

        // Get datasets for workspace
        let datasets = self.workspace_store.list_datasets_for_workspace(workspace_id).await?;

        if datasets.is_empty() {
            return Err(GeoragError::IndexNotBuilt(
                "No datasets in workspace to index".to_string(),
            ));
        }

        tracing::info!(
            workspace_id = %workspace_id,
            dataset_count = datasets.len(),
            "Starting index rebuild"
        );

        // Create embedder from config (default Ollama URL)
        let ollama_url =
            std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let embedder = OllamaEmbedder::new(
            ollama_url,
            &self.embedder_config.model,
            self.embedder_config.dimensions,
        );

        // Create workspace CRS (default to WGS84)
        let workspace_crs = Crs::wgs84();

        // Create IndexBuilder with stores
        let builder = IndexBuilder::new(
            self.spatial_store.clone(),
            self.vector_store.clone(),
            self.document_store.clone(),
            embedder,
            workspace_crs,
        )
        .with_batch_size(32);

        // Perform full rebuild with progress logging
        let result = builder
            .full_rebuild(&datasets, true, |progress| {
                tracing::debug!(
                    phase = ?progress.phase,
                    current = progress.current,
                    total = progress.total,
                    message = %progress.message,
                    "Index rebuild progress"
                );
            })
            .await?;

        tracing::info!(
            workspace_id = %workspace_id,
            chunk_count = result.chunk_count,
            embedding_dim = result.embedding_dim,
            hash = %result.index_hash,
            "Index rebuild completed"
        );

        // Create and store the index state
        let index_state = builder.create_index_state(&result);
        self.set_workspace_index_state(workspace_id, index_state).await;

        Ok(())
    }
}
