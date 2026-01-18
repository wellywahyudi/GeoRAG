use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::dto::{IndexIntegrityResponse, IndexStatusResponse, RebuildResponse, VerifyResponse};
use crate::error::ApiError;
use crate::state::AppState;

/// Get index integrity (legacy endpoint)
pub async fn get_index_integrity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IndexIntegrityResponse>, ApiError> {
    let index_state = state.get_index_state().await?;

    Ok(Json(IndexIntegrityResponse {
        hash: index_state.hash,
        built_at: index_state.built_at,
        embedder: index_state.embedder,
        chunk_count: index_state.chunk_count,
        embedding_dim: index_state.embedding_dim,
    }))
}

/// Verify index integrity (legacy endpoint)
pub async fn verify_index(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let stored_state = state.get_index_state().await?;
    let computed_hash = state.compute_index_hash().await?;

    Ok(Json(VerifyResponse {
        stored_hash: stored_state.hash.clone(),
        computed_hash: computed_hash.clone(),
        matches: stored_state.hash == computed_hash,
    }))
}

/// Trigger index rebuild for a workspace (returns 202 Accepted)
pub async fn rebuild_index(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<(StatusCode, Json<RebuildResponse>), ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Triggering index rebuild");

    let ws_id = workspace_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid workspace ID format"))?;

    let workspace = state.workspace_store.get_workspace(ws_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to check workspace existence");
        ApiError::internal("Failed to verify workspace").with_details(e.to_string())
    })?;

    if workspace.is_none() {
        return Err(ApiError::not_found("Workspace not found"));
    }

    // Check if a rebuild is already in progress
    if state.is_rebuilding(ws_id).await {
        return Err(ApiError::bad_request("Rebuild already in progress")
            .with_details("Wait for the current rebuild to complete or check /index/status"));
    }

    // Start background rebuild task
    state.start_rebuild(ws_id).await;

    let state_clone = state.clone();
    let ws_id_clone = ws_id;
    tokio::spawn(async move {
        tracing::info!(workspace_id = %ws_id_clone, "Starting background index rebuild");

        // Perform the rebuild
        match state_clone.rebuild_index_for_workspace(ws_id_clone).await {
            Ok(_) => {
                tracing::info!(workspace_id = %ws_id_clone, "Index rebuild completed successfully");
            }
            Err(e) => {
                tracing::error!(workspace_id = %ws_id_clone, error = %e, "Index rebuild failed");
                state_clone.set_rebuild_error(ws_id_clone, e.to_string()).await;
            }
        }

        state_clone.finish_rebuild(ws_id_clone).await;
    });

    Ok((StatusCode::ACCEPTED, Json(RebuildResponse::accepted())))
}

/// Get index status for a workspace
pub async fn get_workspace_index_status(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<IndexStatusResponse>, ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Getting index status");

    let ws_id = workspace_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid workspace ID format"))?;

    let workspace = state.workspace_store.get_workspace(ws_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to check workspace existence");
        ApiError::internal("Failed to verify workspace").with_details(e.to_string())
    })?;

    if workspace.is_none() {
        return Err(ApiError::not_found("Workspace not found"));
    }

    let rebuilding = state.is_rebuilding(ws_id).await;
    let index_state = state.get_workspace_index_state(ws_id).await;

    match index_state {
        Some(idx_state) => Ok(Json(IndexStatusResponse {
            built: true,
            rebuilding,
            hash: Some(idx_state.hash),
            built_at: Some(idx_state.built_at),
            chunk_count: Some(idx_state.chunk_count),
            embedder: Some(idx_state.embedder),
        })),
        None => Ok(Json(IndexStatusResponse {
            built: false,
            rebuilding,
            hash: None,
            built_at: None,
            chunk_count: None,
            embedder: None,
        })),
    }
}

