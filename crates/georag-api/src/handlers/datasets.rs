use std::sync::Arc;

use axum::{extract::State, Json};
use georag_core::models::DatasetMeta;

use crate::dto::DatasetInfo;
use crate::error::ApiError;
use crate::state::AppState;

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

fn dataset_meta_to_info(meta: &DatasetMeta) -> DatasetInfo {
    DatasetInfo {
        id: meta.name.clone(),
        geometry_type: format!("{:?}", meta.geometry_type),
        count: meta.feature_count,
    }
}
