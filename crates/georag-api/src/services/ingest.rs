use georag_core::formats::{FormatFeature, FormatRegistry};
use georag_core::models::{Dataset, DatasetId, Feature, FeatureId, Geometry as CoreGeometry};
use std::path::Path;

use crate::error::ApiError;
use crate::state::AppState;

/// Result of ingesting a file
pub struct IngestResult {
    pub dataset_id: DatasetId,
    pub feature_count: usize,
}

/// Service for ingesting datasets
pub struct IngestService;

impl IngestService {
    /// Ingest a file from bytes
    pub async fn ingest_file(
        state: &AppState,
        filename: &str,
        data: &[u8],
    ) -> Result<IngestResult, ApiError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            ApiError::internal("Failed to create temp directory").with_details(e.to_string())
        })?;

        let temp_path = temp_dir.path().join(filename);
        std::fs::write(&temp_path, data).map_err(|e| {
            ApiError::internal("Failed to write temp file").with_details(e.to_string())
        })?;

        Self::ingest_from_path(state, &temp_path, filename).await
    }

    /// Ingest a file from a path
    async fn ingest_from_path(
        state: &AppState,
        path: &Path,
        filename: &str,
    ) -> Result<IngestResult, ApiError> {
        let registry = FormatRegistry::default();

        let reader = registry.detect_format(path).map_err(|e| {
            ApiError::bad_request("Unsupported file format").with_details(e.to_string())
        })?;

        let format_dataset = reader.read(path).await.map_err(|e| {
            ApiError::bad_request("Failed to parse file").with_details(e.to_string())
        })?;

        let dataset = Dataset {
            id: DatasetId(0),
            name: filename.to_string(),
            path: path.to_path_buf(),
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
            ApiError::internal("Failed to store dataset").with_details(e.to_string())
        })?;

        let crs = format_dataset.crs;
        let features: Vec<Feature> = format_dataset
            .features
            .into_iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let geom = f.geometry.as_ref().and_then(CoreGeometry::from_geojson)?;
                Some(Feature::with_geometry(FeatureId(i as u64), geom, f.properties, crs))
            })
            .collect();

        let feature_count = features.len();

        state.spatial_store.store_features(&features).await.map_err(|e| {
            ApiError::internal("Failed to store features").with_details(e.to_string())
        })?;

        tracing::info!(
            dataset_id = dataset_id.0,
            feature_count = feature_count,
            "Successfully ingested dataset"
        );

        Ok(IngestResult { dataset_id, feature_count })
    }
}

fn detect_geometry_type(features: &[FormatFeature]) -> georag_core::models::GeometryType {
    use georag_core::models::GeometryType;

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
