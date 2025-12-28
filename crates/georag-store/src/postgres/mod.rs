pub mod config;
pub mod document;
pub mod index;
pub mod migrations;
pub mod spatial;
pub mod transaction;
pub mod vector;

pub use config::{IndexConfig, MigrationConfig, PoolConfig, PostgresConfig};
pub use index::{IndexStats, RebuildResult, VacuumResult};
pub use migrations::{MigrationError, MigrationManager, MigrationStatus};
pub use transaction::{Transaction, TransactionManager};

use georag_core::error::{GeoragError, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// PostgreSQL storage adapter
pub struct PostgresStore {
    pool: PgPool,
    config: PostgresConfig,
    transaction_manager: TransactionManager,
}

impl PostgresStore {
    /// Create a new PostgreSQL store with the given configuration
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // Validate configuration
        config.validate().map_err(|e| GeoragError::ConfigInvalid {
            key: "database_url".to_string(),
            reason: e.to_string(),
        })?;

        // Create connection pool
        let pool = PgPoolOptions::new()
            .min_connections(config.pool.min_connections)
            .max_connections(config.pool.max_connections)
            .acquire_timeout(config.pool.acquire_timeout)
            .idle_timeout(config.pool.idle_timeout)
            .max_lifetime(config.pool.max_lifetime)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                GeoragError::Serialization(format!("Failed to connect to database: {}", e))
            })?;

        // Test connection by executing a simple query
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Connection test failed: {}", e)))?;

        // Create transaction manager with default 30 second timeout
        let transaction_manager = TransactionManager::new(pool.clone(), Duration::from_secs(30));

        Ok(Self { pool, config, transaction_manager })
    }

    /// Create a new PostgreSQL store and run migrations
    pub async fn with_migrations(config: PostgresConfig) -> Result<Self> {
        let store = Self::new(config).await?;
        store.run_migrations().await?;
        Ok(store)
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> Result<()> {
        let manager = MigrationManager::new(self.pool.clone());
        manager
            .run_migrations()
            .await
            .map_err(|e| GeoragError::Serialization(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// Check migration status
    pub async fn migration_status(&self) -> Result<Vec<MigrationStatus>> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.check_status().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to check migration status: {}", e))
        })
    }

    /// Check if there are pending migrations
    pub async fn has_pending_migrations(&self) -> Result<bool> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.has_pending_migrations().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to check pending migrations: {}", e))
        })
    }

    /// Get the current schema version
    pub async fn current_version(&self) -> Result<Option<i64>> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.current_version().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to get current version: {}", e))
        })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &PostgresConfig {
        &self.config
    }

    /// Perform a health check on the database connection
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Health check failed: {}", e)))?;
        Ok(())
    }

    /// Begin a new transaction with the default timeout (30 seconds)
    ///
    /// Returns a Transaction handle that can be used to execute operations
    /// atomically. The transaction must be explicitly committed or rolled back.
    ///
    pub async fn begin_transaction(&self) -> Result<transaction::Transaction<'_>> {
        self.transaction_manager.begin_transaction().await
    }

    /// Begin a new transaction with a custom timeout
    pub async fn begin_transaction_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<transaction::Transaction<'_>> {
        self.transaction_manager.begin_transaction_with_timeout(timeout).await
    }

    /// Get a reference to the transaction manager
    pub fn transaction_manager(&self) -> &TransactionManager {
        &self.transaction_manager
    }

    /// Rebuild database indexes
    pub async fn rebuild_indexes(
        &self,
        index_name: Option<&str>,
        concurrently: bool,
    ) -> Result<index::RebuildResult> {
        index::rebuild_indexes(&self.pool, index_name, concurrently).await
    }

    /// Get statistics for database indexes
    pub async fn get_index_stats(
        &self,
        index_name: Option<&str>,
    ) -> Result<Vec<index::IndexStats>> {
        index::get_index_stats(&self.pool, index_name).await
    }

    /// Run VACUUM and optionally ANALYZE on database tables
    pub async fn vacuum_analyze(
        &self,
        table_name: Option<&str>,
        analyze: bool,
        full: bool,
    ) -> Result<index::VacuumResult> {
        index::vacuum_analyze(&self.pool, table_name, analyze, full).await
    }
}
