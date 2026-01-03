use crate::cli::StorageBackend;
use anyhow::{Context, Result};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use std::sync::Arc;

pub struct Storage {
    pub spatial: Arc<dyn SpatialStore>,
    pub vector: Arc<dyn VectorStore>,
    pub document: Arc<dyn DocumentStore>,
}

impl Storage {
    pub async fn new(backend: StorageBackend) -> Result<Self> {
        match backend {
            StorageBackend::Memory => Self::new_memory(),
            StorageBackend::Postgres => Self::new_postgres().await,
        }
    }

    /// Create in-memory storage adapters
    fn new_memory() -> Result<Self> {
        Ok(Self {
            spatial: Arc::new(MemorySpatialStore::new()),
            vector: Arc::new(MemoryVectorStore::new()),
            document: Arc::new(MemoryDocumentStore::new()),
        })
    }

    /// Create PostgreSQL storage adapters
    async fn new_postgres() -> Result<Self> {
        let config = PostgresConfig::from_env().context(
            "Failed to load PostgreSQL configuration. Set DATABASE_URL environment variable.",
        )?;
        let store = PostgresStore::with_migrations(config)
            .await
            .context("Failed to connect to PostgreSQL database")?;
        let store = Arc::new(store);

        Ok(Self {
            spatial: store.clone(),
            vector: store.clone(),
            document: store.clone(),
        })
    }

    /// Check if storage has any data
    pub async fn is_empty(&self) -> Result<bool> {
        let datasets = self.spatial.list_datasets().await?;
        let chunk_ids = self.document.list_chunk_ids().await?;
        Ok(datasets.is_empty() && chunk_ids.is_empty())
    }

    /// Clear all data (for --force rebuild)
    pub async fn clear(&self) -> Result<()> {
        // Clear all chunks
        let chunk_ids = self.document.list_chunk_ids().await?;
        if !chunk_ids.is_empty() {
            self.document.delete_chunks(&chunk_ids).await?;
        }

        // Clear all embeddings
        if !chunk_ids.is_empty() {
            self.vector.delete_embeddings(&chunk_ids).await?;
        }

        // Only clear the derived data (chunks and embeddings)
        Ok(())
    }
}
