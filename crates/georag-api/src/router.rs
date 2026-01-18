use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::handlers;
use crate::state::AppState;

/// Create the API router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health
        .route("/health", get(handlers::health_check))

        // Workspaces
        .route("/api/v1/workspaces", post(handlers::create_workspace))
        .route("/api/v1/workspaces", get(handlers::list_workspaces))
        .route("/api/v1/workspaces/:workspace_id", delete(handlers::delete_workspace))

        // Datasets (workspace-scoped)
        .route("/api/v1/workspaces/:workspace_id/datasets", get(handlers::list_datasets_for_workspace))
        .route("/api/v1/workspaces/:workspace_id/datasets/:dataset_id", delete(handlers::delete_dataset))

        // Index (workspace-scoped)
        .route("/api/v1/workspaces/:workspace_id/index/rebuild", post(handlers::rebuild_index))
        .route("/api/v1/workspaces/:workspace_id/index/status", get(handlers::get_workspace_index_status))

        // Legacy routes (backward compatibility)
        .route("/api/v1/query", post(handlers::handle_query))
        .route("/api/v1/datasets", get(handlers::list_datasets))
        .route("/api/v1/ingest", post(handlers::handle_ingest))
        .route("/api/v1/index/integrity", get(handlers::get_index_integrity))
        .route("/api/v1/index/verify", post(handlers::verify_index))

        .with_state(state)
}
