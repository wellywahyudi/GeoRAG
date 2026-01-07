use std::env;
use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use georag_api::routes::create_router;
use georag_api::state::{AppState, EmbedderConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "georag_api=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = env::var("GEORAG_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3001);

    let embedder_model =
        env::var("GEORAG_EMBEDDER_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string());

    let embedder_dim: usize =
        env::var("GEORAG_EMBEDDER_DIM").ok().and_then(|d| d.parse().ok()).unwrap_or(768);

    tracing::info!(
        port = port,
        embedder_model = %embedder_model,
        embedder_dim = embedder_dim,
        "Starting GeoRAG API server"
    );

    // TODO: Add PostgreSQL support via DATABASE_URL env var
    let spatial_store = Arc::new(MemorySpatialStore::new());
    let vector_store = Arc::new(MemoryVectorStore::new());
    let document_store = Arc::new(MemoryDocumentStore::new());

    tracing::info!("Using in-memory storage");

    let state = Arc::new(AppState::new(
        spatial_store,
        vector_store,
        document_store,
        EmbedderConfig {
            model: embedder_model,
            dimensions: embedder_dim,
        },
    ));

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = create_router(state).layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("Listening on {}", addr);
    tracing::info!("CORS enabled for http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
