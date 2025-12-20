//! PostgreSQL storage adapter implementation

pub mod config;
pub mod migrations;
pub mod spatial;
pub mod vector;

pub use config::{PostgresConfig, PoolConfig, MigrationConfig, IndexConfig};
pub use migrations::{MigrationManager, MigrationStatus, MigrationError};

use sqlx::{PgPool, postgres::PgPoolOptions};
use georag_core::error::{Result, GeoragError};

/// PostgreSQL storage adapter
pub struct PostgresStore {
    pool: PgPool,
    config: PostgresConfig,
}

impl PostgresStore {
    /// Create a new PostgreSQL store with the given configuration
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        // Validate configuration
        config.validate()
            .map_err(|e| GeoragError::ConfigInvalid {
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
            .map_err(|e| GeoragError::Serialization(format!("Failed to connect to database: {}", e)))?;

        // Test connection by executing a simple query
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Connection test failed: {}", e)))?;

        Ok(Self { pool, config })
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
        manager.run_migrations().await
            .map_err(|e| GeoragError::Serialization(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// Check migration status
    pub async fn migration_status(&self) -> Result<Vec<MigrationStatus>> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.check_status().await
            .map_err(|e| GeoragError::Serialization(format!("Failed to check migration status: {}", e)))
    }

    /// Check if there are pending migrations
    pub async fn has_pending_migrations(&self) -> Result<bool> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.has_pending_migrations().await
            .map_err(|e| GeoragError::Serialization(format!("Failed to check pending migrations: {}", e)))
    }

    /// Get the current schema version
    pub async fn current_version(&self) -> Result<Option<i64>> {
        let manager = MigrationManager::new(self.pool.clone());
        manager.current_version().await
            .map_err(|e| GeoragError::Serialization(format!("Failed to get current version: {}", e)))
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
}
