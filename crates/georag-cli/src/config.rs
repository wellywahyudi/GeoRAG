use anyhow::{Context, Result};
use georag_core::config::{CliConfigOverrides, LayeredConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Complete configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub postgres: Option<PostgresConfig>,

    #[serde(default)]
    pub embedder: Option<EmbedderConfig>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend: "memory" or "postgres"
    #[serde(default = "default_storage_backend")]
    pub backend: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self { backend: "memory".to_string() }
    }
}

fn default_storage_backend() -> String {
    "memory".to_string()
}

/// PostgreSQL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub host: String,

    #[serde(default = "default_postgres_port")]
    pub port: u16,

    pub database: String,

    #[serde(default = "default_postgres_user")]
    pub user: String,

    /// Password is optional - can use .pgpass or environment variable
    pub password: Option<String>,

    #[serde(default)]
    pub pool: Option<PoolConfig>,
}

fn default_postgres_port() -> u16 {
    5432
}

fn default_postgres_user() -> String {
    "postgres".to_string()
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    #[serde(default = "default_acquire_timeout")]
    pub acquire_timeout: u64,

    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
}

fn default_min_connections() -> u32 {
    2
}

fn default_max_connections() -> u32 {
    10
}

fn default_acquire_timeout() -> u64 {
    30
}

fn default_idle_timeout() -> u64 {
    600
}

/// Embedder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    /// Default embedder (e.g., "ollama:nomic-embed-text")
    pub default: String,
}

impl ConfigFile {
    /// Load configuration from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: ConfigFile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from workspace directory
    pub fn load_from_workspace(workspace_dir: &Path) -> Result<Option<Self>> {
        let config_path = workspace_dir.join(".georag").join("config.toml");

        if !config_path.exists() {
            return Ok(None);
        }

        Self::load(&config_path).map(Some)
    }

    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Create a default configuration file
    pub fn create_default(
        workspace_dir: &Path,
        use_postgres: bool,
        database_url: Option<String>,
    ) -> Result<Self> {
        let postgres = if use_postgres {
            if let Some(url) = database_url {
                // Parse DATABASE_URL
                Some(Self::parse_database_url(&url)?)
            } else {
                // Use defaults
                Some(PostgresConfig {
                    host: "localhost".to_string(),
                    port: 5432,
                    database: "georag".to_string(),
                    user: "postgres".to_string(),
                    password: None,
                    pool: Some(PoolConfig {
                        min_connections: 2,
                        max_connections: 10,
                        acquire_timeout: 30,
                        idle_timeout: 600,
                    }),
                })
            }
        } else {
            None
        };

        let config = ConfigFile {
            storage: StorageConfig {
                backend: if use_postgres { "postgres" } else { "memory" }.to_string(),
            },
            postgres,
            embedder: Some(EmbedderConfig {
                default: "ollama:nomic-embed-text".to_string(),
            }),
        };

        // Save to workspace
        let config_path = workspace_dir.join(".georag").join("config.toml");
        config.save(&config_path)?;

        Ok(config)
    }

    /// Parse DATABASE_URL into PostgresConfig
    fn parse_database_url(url: &str) -> Result<PostgresConfig> {
        // Simple parsing - in production, use url crate
        // Format: postgresql://user:password@host:port/database

        let url = url
            .strip_prefix("postgresql://")
            .or_else(|| url.strip_prefix("postgres://"))
            .context("Invalid DATABASE_URL format")?;

        let (auth, rest) = url.split_once('@').context("Invalid DATABASE_URL: missing @")?;

        let (user, password) = if let Some((u, p)) = auth.split_once(':') {
            (u.to_string(), Some(p.to_string()))
        } else {
            (auth.to_string(), None)
        };

        let (host_port, database) =
            rest.split_once('/').context("Invalid DATABASE_URL: missing /")?;

        let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
            (h.to_string(), p.parse().unwrap_or(5432))
        } else {
            (host_port.to_string(), 5432)
        };

        Ok(PostgresConfig {
            host,
            port,
            database: database.to_string(),
            user,
            password,
            pool: Some(PoolConfig {
                min_connections: 2,
                max_connections: 10,
                acquire_timeout: 30,
                idle_timeout: 600,
            }),
        })
    }
}

/// Load configuration with fallback to environment variables
pub fn load_config_with_fallback(workspace_dir: &Path) -> Result<ConfigFile> {
    // Try to load from file first
    if let Some(config) = ConfigFile::load_from_workspace(workspace_dir)? {
        return Ok(config);
    }

    // Fallback to environment variables
    let database_url = std::env::var("DATABASE_URL").ok();
    let use_postgres = database_url.is_some();

    Ok(ConfigFile {
        storage: StorageConfig {
            backend: if use_postgres { "postgres" } else { "memory" }.to_string(),
        },
        postgres: database_url.and_then(|url| ConfigFile::parse_database_url(&url).ok()),
        embedder: Some(EmbedderConfig {
            default: "ollama:nomic-embed-text".to_string(),
        }),
    })
}

/// Find workspace root directory
pub fn find_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let georag_dir = current.join(".georag");
        if georag_dir.exists() && georag_dir.is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            anyhow::bail!("Not in a GeoRAG workspace. Run 'georag init' first.");
        }
    }
}

// ============================================================================
// Layered configuration loading utilities
// ============================================================================

/// Load layered configuration for a workspace
pub fn load_workspace_config(workspace_root: &PathBuf) -> Result<LayeredConfig> {
    let config_path = workspace_root.join(".georag").join("config.toml");

    let config = LayeredConfig::with_defaults()
        .load_from_file(&config_path)
        .context("Failed to load configuration file")?
        .load_from_env();

    Ok(config)
}

/// Load layered configuration with CLI overrides
pub fn load_workspace_config_with_overrides(
    workspace_root: &PathBuf,
    overrides: CliConfigOverrides,
) -> Result<LayeredConfig> {
    let mut config = load_workspace_config(workspace_root)?;
    config.update_from_cli(overrides);
    Ok(config)
}
