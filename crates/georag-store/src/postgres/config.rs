//! PostgreSQL configuration

use std::time::Duration;
use thiserror::Error;

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required configuration: {0}")]
    Missing(String),

    #[error("Invalid configuration value for {key}: {reason}")]
    Invalid { key: String, reason: String },

    #[error("SSL configuration error: {0}")]
    Ssl(String),
}

/// PostgreSQL connection and behavior configuration
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Database connection URL
    pub database_url: String,
    /// Connection pool configuration
    pub pool: PoolConfig,
    /// Migration configuration
    pub migrations: MigrationConfig,
    /// Index configuration
    pub indexes: IndexConfig,
}

impl PostgresConfig {
    /// Load configuration from environment variables
    ///
    /// Requires DATABASE_URL environment variable to be set.
    /// Other settings use defaults if not specified.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::Missing("DATABASE_URL".to_string()))?;

        // Validate that database_url is not empty
        if database_url.trim().is_empty() {
            return Err(ConfigError::Invalid {
                key: "DATABASE_URL".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        Ok(Self {
            database_url,
            pool: PoolConfig::default(),
            migrations: MigrationConfig::default(),
            indexes: IndexConfig::default(),
        })
    }

    /// Create a new configuration with the given database URL
    pub fn new(database_url: String) -> Result<Self, ConfigError> {
        if database_url.trim().is_empty() {
            return Err(ConfigError::Invalid {
                key: "database_url".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        Ok(Self {
            database_url,
            pool: PoolConfig::default(),
            migrations: MigrationConfig::default(),
            indexes: IndexConfig::default(),
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.database_url.trim().is_empty() {
            return Err(ConfigError::Invalid {
                key: "database_url".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        self.pool.validate()?;

        Ok(())
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Maximum number of connections allowed
    pub max_connections: u32,
    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,
    /// Timeout for idle connections before they are closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl PoolConfig {
    /// Validate pool configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.min_connections > self.max_connections {
            return Err(ConfigError::Invalid {
                key: "pool.min_connections".to_string(),
                reason: format!(
                    "min_connections ({}) cannot be greater than max_connections ({})",
                    self.min_connections, self.max_connections
                ),
            });
        }

        if self.max_connections == 0 {
            return Err(ConfigError::Invalid {
                key: "pool.max_connections".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Whether to automatically run migrations on startup
    pub auto_run: bool,
    /// Name of the migration tracking table
    pub migration_table: String,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_run: false,
            migration_table: "_sqlx_migrations".to_string(),
        }
    }
}

/// Index configuration
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Number of lists for IVFFlat index (None = auto-calculate)
    pub ivfflat_lists: Option<usize>,
    /// Whether to rebuild indexes concurrently (non-blocking)
    pub rebuild_concurrently: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            ivfflat_lists: None,
            rebuild_concurrently: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_valid() {
        let config = PostgresConfig::new("postgresql://localhost/test".to_string());
        assert!(config.is_ok());
    }

    #[test]
    fn test_config_new_empty_url() {
        let config = PostgresConfig::new("".to_string());
        assert!(config.is_err());
        match config {
            Err(ConfigError::Invalid { key, .. }) => {
                assert_eq!(key, "database_url");
            }
            _ => panic!("Expected Invalid error"),
        }
    }

    #[test]
    fn test_config_validate_empty_url() {
        let mut config = PostgresConfig::new("postgresql://localhost/test".to_string()).unwrap();
        config.database_url = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pool_config_default() {
        let pool = PoolConfig::default();
        assert_eq!(pool.min_connections, 2);
        assert_eq!(pool.max_connections, 10);
        assert!(pool.validate().is_ok());
    }

    #[test]
    fn test_pool_config_invalid_min_max() {
        let mut pool = PoolConfig::default();
        pool.min_connections = 20;
        pool.max_connections = 10;
        assert!(pool.validate().is_err());
    }

    #[test]
    fn test_pool_config_zero_max() {
        let mut pool = PoolConfig::default();
        pool.max_connections = 0;
        assert!(pool.validate().is_err());
    }

    #[test]
    fn test_migration_config_default() {
        let migration = MigrationConfig::default();
        assert!(!migration.auto_run);
        assert_eq!(migration.migration_table, "_sqlx_migrations");
    }

    #[test]
    fn test_index_config_default() {
        let index = IndexConfig::default();
        assert!(index.ivfflat_lists.is_none());
        assert!(index.rebuild_concurrently);
    }
}
