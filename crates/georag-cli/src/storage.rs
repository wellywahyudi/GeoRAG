use crate::cli::StorageBackend;
use anyhow::{Context, Result};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use std::sync::Arc;

/// Parse database URL to extract connection details for error messages
fn parse_database_url(url: &str) -> (String, String, String) {
    // Try to parse postgresql://user:pass@host:port/database format
    let host = url
        .split('@')
        .nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').next())
        .unwrap_or("localhost")
        .to_string();

    let port = url
        .split('@')
        .nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').nth(1))
        .unwrap_or("5432")
        .to_string();

    let database = url
        .split('/')
        .next_back()
        .and_then(|s| s.split('?').next())
        .unwrap_or("georag")
        .to_string();

    (host, port, database)
}

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

        let store = PostgresStore::with_migrations(config.clone()).await.map_err(|e| {
            // Parse connection details from DATABASE_URL for better error messages
            let (host, port, database) = parse_database_url(&config.database_url);

            anyhow::anyhow!(
                "Failed to connect to PostgreSQL\n\n\
                    Connection details:\n\
                      Host: {}\n\
                      Port: {}\n\
                      Database: {}\n\n\
                    Remediation:\n\
                      1. Ensure PostgreSQL is running\n\
                      2. Check DATABASE_URL environment variable\n\
                      3. Verify credentials and database exists\n\
                      4. Test connection: psql {}\n\n\
                    Error: {}",
                host,
                port,
                database,
                config.database_url,
                e
            )
        })?;
        let store = Arc::new(store);

        Ok(Self {
            spatial: store.clone(),
            vector: store.clone(),
            document: store.clone(),
        })
    }

    /// Check if storage has any data
    #[allow(dead_code)]
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
