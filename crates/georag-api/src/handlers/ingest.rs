use std::sync::Arc;

use axum::{extract::Multipart, extract::State, Json};

use crate::dto::IngestResponse;
use crate::error::ApiError;
use crate::services::IngestService;
use crate::state::AppState;

pub async fn handle_ingest(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IngestResponse>, ApiError> {
    tracing::info!("Processing ingest request");

    let (filename, data) = extract_file(&mut multipart).await?;

    tracing::info!(filename = %filename, size = data.len(), "Received file for ingestion");

    let result = IngestService::ingest_file(&state, &filename, &data).await?;

    Ok(Json(IngestResponse::success(
        result.dataset_id.0,
        &filename,
        result.feature_count,
    )))
}

async fn extract_file(multipart: &mut Multipart) -> Result<(String, Vec<u8>), ApiError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::bad_request("Failed to parse multipart form").with_details(e.to_string())
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let filename = field.file_name().unwrap_or("upload.geojson").to_string();
            let data = field.bytes().await.map_err(|e| {
                ApiError::bad_request("Failed to read file data").with_details(e.to_string())
            })?;
            return Ok((filename, data.to_vec()));
        }
    }

    Err(ApiError::bad_request("No file provided")
        .with_details("Expected a 'file' field in the multipart form"))
}
