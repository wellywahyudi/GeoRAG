use std::env;

/// API server configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub port: u16,
    pub cors_origin: String,
    pub database_url: Option<String>,
    pub embedder: EmbedderConfig,
}

/// Embedder configuration
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

impl ApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let port = env::var("GEORAG_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3001);

        let cors_origin =
            env::var("GEORAG_CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let database_url = env::var("DATABASE_URL").ok();

        let embedder = EmbedderConfig {
            model: env::var("GEORAG_EMBEDDER_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string()),
            dimensions: env::var("GEORAG_EMBEDDER_DIM")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(768),
        };

        Self {
            port,
            cors_origin,
            database_url,
            embedder,
        }
    }

    /// Get the server bind address
    pub fn bind_address(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }

    /// Check if PostgreSQL storage is configured
    pub fn uses_postgres(&self) -> bool {
        self.database_url.is_some()
    }
}
