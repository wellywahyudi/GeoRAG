use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use georag_core::config::{parse_distance_unit, parse_validity_mode};
use georag_core::models::WorkspaceConfig;

use crate::dto::{CreateWorkspaceRequest, DeleteResponse, WorkspaceResponse};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), ApiError> {
    tracing::info!(name = %request.name, crs = request.crs, "Creating workspace");

    let distance_unit = request
        .distance_unit
        .as_deref()
        .map(parse_distance_unit)
        .transpose()
        .map_err(|e| ApiError::bad_request("Invalid distance unit").with_details(e.to_string()))?
        .unwrap_or_default();

    let geometry_validity = request
        .geometry_validity
        .as_deref()
        .map(parse_validity_mode)
        .transpose()
        .map_err(|e| ApiError::bad_request("Invalid validity mode").with_details(e.to_string()))?
        .unwrap_or_default();

    let config = WorkspaceConfig {
        crs: request.crs,
        distance_unit,
        geometry_validity,
    };

    let id = state
        .workspace_store
        .create_workspace(&request.name, &config)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create workspace");
            ApiError::internal("Failed to create workspace").with_details(e.to_string())
        })?;

    let workspace = state
        .workspace_store
        .get_workspace(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get workspace after creation");
            ApiError::internal("Failed to retrieve created workspace").with_details(e.to_string())
        })?
        .ok_or_else(|| ApiError::internal("Workspace not found after creation"))?;

    let response = WorkspaceResponse {
        id: workspace.id.to_string(),
        name: workspace.name,
        crs: workspace.crs,
        distance_unit: format!("{:?}", workspace.distance_unit),
        geometry_validity: format!("{:?}", workspace.geometry_validity),
        created_at: workspace.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_workspaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkspaceResponse>>, ApiError> {
    tracing::info!("Listing workspaces");

    let workspaces = state.workspace_store.list_workspaces().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list workspaces");
        ApiError::internal("Failed to list workspaces").with_details(e.to_string())
    })?;

    let responses: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .map(|w| WorkspaceResponse {
            id: w.id.to_string(),
            name: w.name,
            crs: w.crs,
            distance_unit: format!("{:?}", w.distance_unit),
            geometry_validity: format!("{:?}", w.geometry_validity),
            created_at: w.created_at,
        })
        .collect();

    Ok(Json(responses))
}

pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Deleting workspace");

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

    state.workspace_store.delete_workspace(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete workspace");
        ApiError::internal("Failed to delete workspace").with_details(e.to_string())
    })?;

    Ok(Json(DeleteResponse::success("workspace", &workspace_id)))
}
