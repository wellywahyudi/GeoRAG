use std::sync::Arc;

use axum::{extract::State, Json};

use crate::dto::{IndexIntegrityResponse, VerifyResponse};
use crate::error::ApiError;
use crate::state::AppState;

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
