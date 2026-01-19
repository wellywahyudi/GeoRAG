use geojson::{Feature, FeatureCollection, Geometry};
use georag_core::models::{Crs, Geometry as CoreGeometry, SpatialFilter, SpatialPredicate};
use georag_core::llm::OllamaEmbedder;
use georag_retrieval::{QueryPlan, QueryResult, RetrievalPipeline, SourceReference};
use serde_json::{Map, Value as JsonValue};

use crate::config::EmbedderConfig;
use crate::dto::QueryRequest;
use crate::error::ApiError;
use crate::state::AppState;

/// Service for executing queries
pub struct QueryService;

impl QueryService {
    /// Execute a query and return GeoJSON FeatureCollection
    pub async fn execute(
        state: &AppState,
        request: &QueryRequest,
        embedder_config: &EmbedderConfig,
    ) -> Result<FeatureCollection, ApiError> {
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
            OllamaEmbedder::localhost(&embedder_config.model, embedder_config.dimensions);

        let pipeline = RetrievalPipeline::new(
            state.spatial_store.clone(),
            state.vector_store.clone(),
            state.document_store.clone(),
            embedder,
        );

        let result = pipeline.execute(&query_plan).await.map_err(|e| {
            tracing::error!(error = %e, "Query execution failed");
            ApiError::internal("Query execution failed").with_details(e.to_string())
        })?;

        Ok(Self::to_geojson(&result, state).await)
    }

    /// Convert query results to GeoJSON
    async fn to_geojson(result: &QueryResult, state: &AppState) -> FeatureCollection {
        let mut features = Vec::new();

        for source in &result.sources {
            let geometry = Self::get_geometry_for_source(source, state).await;

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

    async fn get_geometry_for_source(
        source: &SourceReference,
        state: &AppState,
    ) -> Option<Geometry> {
        let feature_id = source.feature_id?;
        let feature = state.spatial_store.get_feature(feature_id).await.ok()??;
        let geom = feature.geometry?;
        let geom_value = geom.to_geojson();
        geojson::Geometry::from_json_value(geom_value).ok()
    }
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
