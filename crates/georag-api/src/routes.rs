use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use geojson::{Feature, FeatureCollection, Geometry};
use georag_core::formats::FormatFeature;
use georag_core::models::Geometry as CoreGeometry;
use georag_core::models::{Crs, DatasetMeta, SpatialFilter, SpatialPredicate};
use georag_llm::OllamaEmbedder;
use georag_retrieval::{QueryPlan, QueryResult, RetrievalPipeline, SourceReference};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub text: String,
    pub bbox: Option<[f64; 4]>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    10
}

#[derive(Debug, Serialize)]
pub struct DatasetInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_id: Option<u64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexIntegrityResponse {
    pub hash: String,
    pub built_at: DateTime<Utc>,
    pub embedder: String,
    pub chunk_count: usize,
    pub embedding_dim: usize,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub stored_hash: String,
    pub computed_hash: String,
    pub matches: bool,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/query", post(handle_query))
        .route("/api/v1/datasets", get(handle_list_datasets))
        .route("/api/v1/ingest", post(handle_ingest))
        .route("/api/v1/index/integrity", get(get_index_integrity))
        .route("/api/v1/index/verify", post(verify_index))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "georag-api"
    }))
}

async fn handle_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        query = %request.text,
        top_k = request.top_k,
        has_bbox = request.bbox.is_some(),
        "Processing query request"
    );

    let mut query_plan = QueryPlan::new(&request.text)
        .with_top_k(request.top_k)
        .with_semantic_rerank(true);

    if let Some(bbox) = request.bbox {
        let spatial_filter = SpatialFilter {
            predicate: SpatialPredicate::BoundingBox,
            geometry: Some(bbox_to_polygon(&bbox)),
            distance: None,
            crs: Crs::wgs84(),
        };
        query_plan = query_plan.with_spatial_filter(spatial_filter);
    }

    let embedder =
        OllamaEmbedder::localhost(&state.embedder_config.model, state.embedder_config.dimensions);

    let pipeline = RetrievalPipeline::new(
        state.spatial_store.clone(),
        state.vector_store.clone(),
        state.document_store.clone(),
        embedder,
    );

    let result = pipeline.execute(&query_plan).await.map_err(|e| {
        tracing::error!(error = %e, "Query execution failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Query execution failed".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let feature_collection = query_result_to_geojson(&result, &state).await;
    Ok(Json(feature_collection))
}

async fn handle_list_datasets(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Listing datasets");

    let datasets = state.spatial_store.list_datasets().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list datasets");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list datasets".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let dataset_infos: Vec<DatasetInfo> = datasets.iter().map(dataset_meta_to_info).collect();
    Ok(Json(dataset_infos))
}

async fn handle_ingest(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Processing ingest request");

    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Failed to parse multipart form".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let filename = field.file_name().unwrap_or("upload.geojson").to_string();
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Failed to read file data".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
            })?;
            file_data = Some((filename, data.to_vec()));
            break;
        }
    }

    let (filename, data) = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No file provided".to_string(),
                details: Some("Expected a 'file' field in the multipart form".to_string()),
            }),
        )
    })?;

    tracing::info!(filename = %filename, size = data.len(), "Received file for ingestion");

    let temp_dir = tempfile::tempdir().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create temp directory".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let temp_path = temp_dir.path().join(&filename);
    std::fs::write(&temp_path, &data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to write temp file".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let registry = georag_core::formats::FormatRegistry::default();

    let reader = registry.detect_format(&temp_path).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Unsupported file format".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let format_dataset = reader.read(&temp_path).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Failed to parse file".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let dataset = georag_core::models::Dataset {
        id: georag_core::models::DatasetId(0),
        name: filename.clone(),
        path: temp_path.clone(),
        geometry_type: detect_geometry_type(&format_dataset.features),
        feature_count: format_dataset.features.len(),
        crs: format_dataset.crs,
        format: georag_core::models::dataset::FormatMetadata {
            format_name: format_dataset.format_metadata.format_name.clone(),
            format_version: format_dataset.format_metadata.format_version.clone(),
            layer_name: format_dataset.format_metadata.layer_name.clone(),
            page_count: format_dataset.format_metadata.page_count,
            paragraph_count: format_dataset.format_metadata.paragraph_count,
            extraction_method: format_dataset.format_metadata.extraction_method.clone(),
            spatial_association: None,
        },
        added_at: chrono::Utc::now(),
    };

    let dataset_id = state.spatial_store.store_dataset(&dataset).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to store dataset".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let crs = format_dataset.crs;
    let features: Vec<georag_core::models::Feature> = format_dataset
        .features
        .into_iter()
        .enumerate()
        .filter_map(|(i, f)| {
            // Convert serde_json::Value to typed Geometry
            let geom = f.geometry.as_ref().and_then(CoreGeometry::from_geojson)?;
            Some(georag_core::models::Feature::with_geometry(
                georag_core::models::FeatureId(i as u64),
                geom,
                f.properties,
                crs,
            ))
        })
        .collect();

    let feature_count = features.len();

    state.spatial_store.store_features(&features).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to store features".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    tracing::info!(
        dataset_id = dataset_id.0,
        feature_count = feature_count,
        "Successfully ingested dataset"
    );

    Ok(Json(IngestResponse {
        success: true,
        dataset_id: Some(dataset_id.0),
        message: format!("Successfully ingested {} with {} features", filename, feature_count),
    }))
}

fn bbox_to_polygon(bbox: &[f64; 4]) -> CoreGeometry {
    let [min_lng, min_lat, max_lng, max_lat] = *bbox;
    CoreGeometry::polygon(vec![vec![
        [min_lng, min_lat],
        [max_lng, min_lat],
        [max_lng, max_lat],
        [min_lng, max_lat],
        [min_lng, min_lat],
    ]])
}

fn dataset_meta_to_info(meta: &DatasetMeta) -> DatasetInfo {
    DatasetInfo {
        id: meta.name.clone(),
        geometry_type: format!("{:?}", meta.geometry_type),
        count: meta.feature_count,
    }
}

fn detect_geometry_type(features: &[FormatFeature]) -> georag_core::models::dataset::GeometryType {
    use georag_core::models::dataset::GeometryType;

    for feature in features {
        if let Some(ref geometry) = feature.geometry {
            if let Some(geom_type) = geometry.get("type").and_then(|t| t.as_str()) {
                return match geom_type {
                    "Point" => GeometryType::Point,
                    "LineString" => GeometryType::LineString,
                    "Polygon" => GeometryType::Polygon,
                    "MultiPoint" => GeometryType::MultiPoint,
                    "MultiLineString" => GeometryType::MultiLineString,
                    "MultiPolygon" => GeometryType::MultiPolygon,
                    "GeometryCollection" => GeometryType::GeometryCollection,
                    _ => GeometryType::Point,
                };
            }
        }
    }
    GeometryType::Point
}

async fn query_result_to_geojson(result: &QueryResult, state: &AppState) -> FeatureCollection {
    let mut features = Vec::new();

    for source in &result.sources {
        let geometry = get_geometry_for_source(source, state).await;

        let mut properties = Map::new();
        properties.insert("score".to_string(), JsonValue::from(source.score));
        properties.insert("excerpt".to_string(), JsonValue::from(source.excerpt.clone()));
        properties
            .insert("document_path".to_string(), JsonValue::from(source.document_path.clone()));
        properties.insert("chunk_id".to_string(), JsonValue::from(source.chunk_id.0));

        if let Some(feature_id) = source.feature_id {
            properties.insert("feature_id".to_string(), JsonValue::from(feature_id.0));
        }

        if let Some(page) = source.page {
            properties.insert("page".to_string(), JsonValue::from(page));
        }

        features.push(Feature {
            geometry,
            properties: Some(properties),
            id: None,
            bbox: None,
            foreign_members: None,
        });
    }

    FeatureCollection {
        features,
        bbox: None,
        foreign_members: None,
    }
}

async fn get_geometry_for_source(source: &SourceReference, state: &AppState) -> Option<Geometry> {
    let feature_id = source.feature_id?;
    let feature = state.spatial_store.get_feature(feature_id).await.ok()??;
    let geom = feature.geometry?;
    let geom_value = geom.to_geojson();
    geojson::Geometry::from_json_value(geom_value).ok()
}

/// GET /api/v1/index/integrity - Returns current index state
async fn get_index_integrity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IndexIntegrityResponse>, (StatusCode, Json<ErrorResponse>)> {
    let index_state = state.get_index_state().await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Index not found".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    Ok(Json(IndexIntegrityResponse {
        hash: index_state.hash,
        built_at: index_state.built_at,
        embedder: index_state.embedder,
        chunk_count: index_state.chunk_count,
        embedding_dim: index_state.embedding_dim,
    }))
}

/// POST /api/v1/index/verify - Recompute hash and verify integrity
async fn verify_index(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VerifyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let stored_state = state.get_index_state().await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Index not found".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    let computed_hash = state.compute_index_hash().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to compute hash".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    Ok(Json(VerifyResponse {
        stored_hash: stored_state.hash.clone(),
        computed_hash: computed_hash.clone(),
        matches: stored_state.hash == computed_hash,
    }))
}
