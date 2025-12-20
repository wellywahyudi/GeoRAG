//! Storage backend factory and management

use crate::cli::StorageBackend;
use anyhow::{Context, Result};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::sync::Arc;

/// Storage container that holds all storage adapters
pub struct Storage {
    pub spatial: Arc<dyn SpatialStore>,
    pub vector: Arc<dyn VectorStore>,
    pub document: Arc<dyn DocumentStore>,
}

impl Storage {
    /// Create storage adapters based on the selected backend
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
        // Load PostgreSQL configuration from environment
        let config = PostgresConfig::from_env()
            .context("Failed to load PostgreSQL configuration. Set DATABASE_URL environment variable.")?;

        // Create PostgreSQL store with migrations
        let store = PostgresStore::with_migrations(config)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        // Wrap in Arc for shared ownership
        let store = Arc::new(store);

        Ok(Self {
            spatial: store.clone(),
            vector: store.clone(),
            document: store.clone(),
        })
    }
}
