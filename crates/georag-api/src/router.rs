use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers;
use crate::state::AppState;

/// Create the API router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/query", post(handlers::handle_query))
        .route("/api/v1/datasets", get(handlers::list_datasets))
        .route("/api/v1/ingest", post(handlers::handle_ingest))
        .route("/api/v1/index/integrity", get(handlers::get_index_integrity))
        .route("/api/v1/index/verify", post(handlers::verify_index))
        .with_state(state)
}
