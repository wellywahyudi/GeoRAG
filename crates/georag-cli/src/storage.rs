use crate::cli::StorageBackend;
use anyhow::{Context, Result};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use std::sync::Arc;

pub struct Storage {
    pub spatial: Arc<dyn SpatialStore>,
    #[allow(dead_code)]
    pub vector: Arc<dyn VectorStore>,
    #[allow(dead_code)]
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
}
