use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use georag_store::memory::{
    MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore, MemoryWorkspaceStore,
};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore, WorkspaceStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use georag_api::{create_router, ApiConfig, AppState};

#[tokio::main]
async fn main() {
    init_tracing();

    let config = ApiConfig::from_env();

    tracing::info!(
        port = config.port,
        embedder_model = %config.embedder.model,
        embedder_dim = config.embedder.dimensions,
        "Starting GeoRAG API server"
    );

    let (spatial_store, vector_store, document_store, workspace_store) =
        init_storage(&config).await;

    let state = Arc::new(AppState::new(
        spatial_store,
        vector_store,
        document_store,
        workspace_store,
        config.embedder.clone(),
    ));

    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = create_router(state).layer(cors);

    let addr = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Listening on {}", addr);
    tracing::info!("CORS enabled for {}", config.cors_origin);

    axum::serve(listener, app).await.unwrap();
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "georag_api=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn init_storage(
    config: &ApiConfig,
) -> (
    Arc<dyn SpatialStore>,
    Arc<dyn VectorStore>,
    Arc<dyn DocumentStore>,
    Arc<dyn WorkspaceStore>,
) {
    match &config.database_url {
        Some(database_url) => {
            tracing::info!("DATABASE_URL found, connecting to PostgreSQL...");
            match init_postgres_storage(database_url).await {
                Ok(store) => {
                    tracing::info!("Connected to PostgreSQL");
                    (store.clone(), store.clone(), store.clone(), store)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to PostgreSQL: {}", e);
                    tracing::error!(
                        "Remediation:\n\
                        1. Ensure PostgreSQL is running\n\
                        2. Verify DATABASE_URL is correct\n\
                        3. Check that the database exists and is accessible"
                    );
                    std::process::exit(1);
                }
            }
        }
        None => {
            tracing::info!("Using in-memory storage (set DATABASE_URL for PostgreSQL)");
            (
                Arc::new(MemorySpatialStore::new()),
                Arc::new(MemoryVectorStore::new()),
                Arc::new(MemoryDocumentStore::new()),
                Arc::new(MemoryWorkspaceStore::new()),
            )
        }
    }
}

async fn init_postgres_storage(database_url: &str) -> Result<Arc<PostgresStore>, String> {
    let config = PostgresConfig::from_database_url(database_url)
        .map_err(|e| format!("Invalid DATABASE_URL: {}", e))?;

    PostgresStore::with_migrations(config)
        .await
        .map(Arc::new)
        .map_err(|e| format!("Connection failed: {}", e))
}
