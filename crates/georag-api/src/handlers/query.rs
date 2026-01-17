use std::sync::Arc;

use axum::{extract::State, Json};
use geojson::FeatureCollection;

use crate::dto::QueryRequest;
use crate::error::ApiError;
use crate::services::QueryService;
use crate::state::AppState;

pub async fn handle_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<FeatureCollection>, ApiError> {
    tracing::info!(
        query = %request.text,
        top_k = request.top_k,
        has_bbox = request.bbox.is_some(),
        "Processing query request"
    );

    let result = QueryService::execute(&state, &request, &state.embedder_config).await?;

    Ok(Json(result))
}
