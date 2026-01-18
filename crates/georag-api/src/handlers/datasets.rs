use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use georag_core::models::{DatasetId, DatasetMeta};

use crate::dto::{DatasetInfo, DatasetResponse, DeleteResponse};
use crate::error::ApiError;
use crate::state::AppState;

/// List all datasets (legacy endpoint)
pub async fn list_datasets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DatasetInfo>>, ApiError> {
    tracing::info!("Listing datasets");

    let datasets = state.spatial_store.list_datasets().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list datasets");
        ApiError::internal("Failed to list datasets").with_details(e.to_string())
    })?;

    let infos: Vec<DatasetInfo> = datasets.iter().map(dataset_meta_to_info).collect();
    Ok(Json(infos))
}

/// List datasets for a specific workspace
pub async fn list_datasets_for_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<DatasetResponse>>, ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Listing datasets for workspace");

    let id = workspace_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid workspace ID format"))?;

    let workspace = state.workspace_store.get_workspace(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to check workspace existence");
        ApiError::internal("Failed to verify workspace").with_details(e.to_string())
    })?;

    if workspace.is_none() {
        return Err(ApiError::not_found("Workspace not found"));
    }

    let datasets = state.workspace_store.list_datasets_for_workspace(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list datasets for workspace");
        ApiError::internal("Failed to list datasets").with_details(e.to_string())
    })?;

    let responses: Vec<DatasetResponse> =
        datasets.into_iter().map(dataset_meta_to_response).collect();
    Ok(Json(responses))
}

/// Delete a dataset within a workspace
pub async fn delete_dataset(
    State(state): State<Arc<AppState>>,
    Path((workspace_id, dataset_id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, ApiError> {
    tracing::info!(workspace_id = %workspace_id, dataset_id = %dataset_id, "Deleting dataset");

    let ws_id = workspace_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid workspace ID format"))?;

    let ds_id: u64 = dataset_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid dataset ID format"))?;

    let workspace = state.workspace_store.get_workspace(ws_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to check workspace existence");
        ApiError::internal("Failed to verify workspace").with_details(e.to_string())
    })?;

    if workspace.is_none() {
        return Err(ApiError::not_found("Workspace not found"));
    }

    state
        .workspace_store
        .delete_dataset_in_workspace(ws_id, DatasetId(ds_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete dataset");
            ApiError::internal("Failed to delete dataset").with_details(e.to_string())
        })?;

    Ok(Json(DeleteResponse::success("dataset", &dataset_id)))
}

fn dataset_meta_to_info(meta: &DatasetMeta) -> DatasetInfo {
    DatasetInfo {
        id: meta.name.clone(),
        geometry_type: format!("{:?}", meta.geometry_type),
        count: meta.feature_count,
    }
}

fn dataset_meta_to_response(meta: DatasetMeta) -> DatasetResponse {
    DatasetResponse {
        id: meta.id.0,
        name: meta.name,
        geometry_type: format!("{:?}", meta.geometry_type),
        feature_count: meta.feature_count,
        crs: meta.crs,
        added_at: meta.added_at,
    }
}
