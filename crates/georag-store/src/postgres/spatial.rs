use async_trait::async_trait;
use georag_core::error::{GeoragError, Result};
use georag_core::models::{
    Dataset, DatasetId, DatasetMeta, Feature, FeatureId, Geometry, GeometryType, SpatialFilter,
    SpatialPredicate,
};
use sqlx::Row;
use uuid::Uuid;

use super::PostgresStore;
use crate::ports::SpatialStore;

#[async_trait]
impl SpatialStore for PostgresStore {
    async fn store_dataset(&self, dataset: &Dataset) -> Result<DatasetId> {
        // For now, we'll use a default workspace_id
        // In a full implementation, this would come from the dataset or context
        let workspace_id = Uuid::new_v4();

        // First, ensure workspace exists (create a default one if needed)
        sqlx::query(
            r#"
            INSERT INTO workspaces (id, name, crs, distance_unit, geometry_validity)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (name) DO NOTHING
            "#,
        )
        .bind(workspace_id)
        .bind("default")
        .bind(format!("EPSG:{}", dataset.crs))
        .bind("Meters")
        .bind("Lenient")
        .execute(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to create workspace: {}", e)))?;

        // Get the workspace_id (either the one we just created or existing)
        let workspace_id: Uuid = sqlx::query_scalar("SELECT id FROM workspaces WHERE name = $1")
            .bind("default")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to get workspace: {}", e)))?;

        // Convert DatasetId to UUID
        let dataset_uuid = Uuid::from_u128(dataset.id.0 as u128);

        // Convert geometry type to string
        let geometry_type_str = match dataset.geometry_type {
            GeometryType::Point => "Point",
            GeometryType::LineString => "LineString",
            GeometryType::Polygon => "Polygon",
            GeometryType::MultiPoint => "MultiPoint",
            GeometryType::MultiLineString => "MultiLineString",
            GeometryType::MultiPolygon => "MultiPolygon",
            GeometryType::GeometryCollection | GeometryType::Mixed => "GeometryCollection",
        };

        // Insert dataset
        sqlx::query(
            r#"
            INSERT INTO datasets (id, workspace_id, name, source_path, format, crs, geometry_type, feature_count, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (workspace_id, name) DO UPDATE
            SET source_path = EXCLUDED.source_path,
                format = EXCLUDED.format,
                crs = EXCLUDED.crs,
                geometry_type = EXCLUDED.geometry_type,
                feature_count = EXCLUDED.feature_count,
                metadata = EXCLUDED.metadata
            "#
        )
        .bind(dataset_uuid)
        .bind(workspace_id)
        .bind(&dataset.name)
        .bind(dataset.path.to_string_lossy().to_string())
        .bind("geojson") // Default format
        .bind(format!("EPSG:{}", dataset.crs))
        .bind(geometry_type_str)
        .bind(dataset.feature_count as i32)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to store dataset: {}", e)))?;

        Ok(dataset.id)
    }

    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>> {
        let dataset_uuid = Uuid::from_u128(id.0 as u128);

        let row = sqlx::query(
            r#"
            SELECT id, name, source_path, crs, geometry_type, feature_count, created_at
            FROM datasets
            WHERE id = $1
            "#,
        )
        .bind(dataset_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get dataset: {}", e)))?;

        match row {
            Some(row) => {
                let crs_str: String = row.get("crs");
                let crs = crs_str
                    .strip_prefix("EPSG:")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(4326);

                let geometry_type_str: String = row.get("geometry_type");
                let geometry_type = match geometry_type_str.as_str() {
                    "Point" => GeometryType::Point,
                    "LineString" => GeometryType::LineString,
                    "Polygon" => GeometryType::Polygon,
                    "MultiPoint" => GeometryType::MultiPoint,
                    "MultiLineString" => GeometryType::MultiLineString,
                    "MultiPolygon" => GeometryType::MultiPolygon,
                    _ => GeometryType::GeometryCollection,
                };

                let dataset = Dataset {
                    id,
                    name: row.get("name"),
                    path: std::path::PathBuf::from(row.get::<String, _>("source_path")),
                    geometry_type,
                    feature_count: row.get::<i32, _>("feature_count") as usize,
                    crs,
                    format: georag_core::models::dataset::FormatMetadata {
                        format_name: "GeoJSON".to_string(), // Default for now
                        format_version: None,
                        layer_name: None,
                        page_count: None,
                        paragraph_count: None,
                        extraction_method: None,
                        spatial_association: None,
                    },
                    added_at: row.get("created_at"),
                };

                Ok(Some(dataset))
            }
            None => Ok(None),
        }
    }

    async fn list_datasets(&self) -> Result<Vec<DatasetMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, crs, geometry_type, feature_count, created_at
            FROM datasets
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to list datasets: {}", e)))?;

        let datasets = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                let id = DatasetId(uuid.as_u128() as u64);

                let crs_str: String = row.get("crs");
                let crs = crs_str
                    .strip_prefix("EPSG:")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(4326);

                let geometry_type_str: String = row.get("geometry_type");
                let geometry_type = match geometry_type_str.as_str() {
                    "Point" => GeometryType::Point,
                    "LineString" => GeometryType::LineString,
                    "Polygon" => GeometryType::Polygon,
                    "MultiPoint" => GeometryType::MultiPoint,
                    "MultiLineString" => GeometryType::MultiLineString,
                    "MultiPolygon" => GeometryType::MultiPolygon,
                    _ => GeometryType::GeometryCollection,
                };

                DatasetMeta {
                    id,
                    name: row.get("name"),
                    geometry_type,
                    feature_count: row.get::<i32, _>("feature_count") as usize,
                    crs,
                    added_at: row.get("created_at"),
                }
            })
            .collect();

        Ok(datasets)
    }

    async fn delete_dataset(&self, id: DatasetId) -> Result<()> {
        let dataset_uuid = Uuid::from_u128(id.0 as u128);

        sqlx::query("DELETE FROM datasets WHERE id = $1")
            .bind(dataset_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to delete dataset: {}", e)))?;

        Ok(())
    }

    async fn store_features(&self, features: &[Feature]) -> Result<()> {
        if features.is_empty() {
            return Ok(());
        }

        // Start a transaction for batch insert
        let mut tx = self.pool.begin().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to begin transaction: {}", e))
        })?;

        // Get or create a default dataset for features
        // In a real implementation, features would be associated with a specific dataset
        // through the API call context or feature metadata
        let dataset_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT id FROM datasets
            WHERE name = 'default_features'
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to query dataset: {}", e)))?
        .unwrap_or_else(|| {
            // If no default dataset exists, we'll create one on-the-fly
            // This is a workaround - in production, features should always have an explicit dataset
            Uuid::new_v4()
        });

        // If we generated a new UUID, we need to create the dataset
        let dataset_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM datasets WHERE id = $1)")
                .bind(dataset_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| {
                    GeoragError::Serialization(format!("Failed to check dataset existence: {}", e))
                })?;

        if !dataset_exists {
            // Get or create default workspace
            let workspace_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO workspaces (name, crs, distance_unit, geometry_validity)
                VALUES ('default', 'EPSG:4326', 'Meters', 'Lenient')
                ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                RETURNING id
                "#,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                GeoragError::Serialization(format!("Failed to create workspace: {}", e))
            })?;

            // Create default dataset
            sqlx::query(
                r#"
                INSERT INTO datasets (id, workspace_id, name, source_path, format, crs, geometry_type, feature_count)
                VALUES ($1, $2, 'default_features', '/tmp/default', 'geojson', 'EPSG:4326', 'GeometryCollection', 0)
                "#
            )
            .bind(dataset_id)
            .bind(workspace_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to create default dataset: {}", e)))?;
        }

        for feature in features {
            let feature_uuid = Uuid::from_u128(feature.id.0 as u128);

            // Convert geometry to GeoJSON string
            let geometry_json = serde_json::to_string(&feature.geometry).map_err(|e| {
                GeoragError::Serialization(format!("Failed to serialize geometry: {}", e))
            })?;

            // Convert properties to JSONB
            let properties_json = serde_json::to_value(&feature.properties).map_err(|e| {
                GeoragError::Serialization(format!("Failed to serialize properties: {}", e))
            })?;

            sqlx::query(
                r#"
                INSERT INTO features (id, dataset_id, feature_id, geometry, properties)
                VALUES ($1, $2, $3, ST_GeomFromGeoJSON($4), $5)
                ON CONFLICT (dataset_id, feature_id) DO UPDATE
                SET geometry = EXCLUDED.geometry,
                    properties = EXCLUDED.properties
                "#,
            )
            .bind(feature_uuid)
            .bind(dataset_id)
            .bind(feature.id.0.to_string())
            .bind(geometry_json)
            .bind(properties_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to store feature: {}", e)))?;
        }

        tx.commit().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn spatial_query(&self, filter: &SpatialFilter) -> Result<Vec<Feature>> {
        // Build the WHERE clause based on the spatial predicate
        let (where_clause, needs_geometry, needs_distance) = match filter.predicate {
            SpatialPredicate::Within => {
                ("ST_Within(geometry, ST_GeomFromGeoJSON($1))", true, false)
            }
            SpatialPredicate::Intersects => {
                ("ST_Intersects(geometry, ST_GeomFromGeoJSON($1))", true, false)
            }
            SpatialPredicate::Contains => {
                ("ST_Contains(geometry, ST_GeomFromGeoJSON($1))", true, false)
            }
            SpatialPredicate::BoundingBox => ("geometry && ST_GeomFromGeoJSON($1)", true, false),
            SpatialPredicate::DWithin => (
                "ST_DWithin(geometry::geography, ST_GeomFromGeoJSON($1)::geography, $2)",
                true,
                true,
            ),
        };

        // Check if we have the required parameters
        if needs_geometry && filter.geometry.is_none() {
            return Err(GeoragError::Serialization(
                "Spatial query requires geometry parameter".to_string(),
            ));
        }

        // For distance queries, we need both geometry and distance
        if needs_distance && (filter.geometry.is_none() || filter.distance.is_none()) {
            return Err(GeoragError::Serialization(
                "Distance query requires both geometry and distance parameters".to_string(),
            ));
        }

        let geometry_json =
            serde_json::to_string(filter.geometry.as_ref().unwrap()).map_err(|e| {
                GeoragError::Serialization(format!("Failed to serialize geometry: {}", e))
            })?;

        let query_str = format!(
            r#"
            SELECT id, feature_id, ST_AsGeoJSON(geometry) as geometry, properties
            FROM features
            WHERE {}
            "#,
            where_clause
        );

        let rows = sqlx::query(&query_str)
            .bind(geometry_json)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                GeoragError::Serialization(format!("Failed to execute spatial query: {}", e))
            })?;

        let features = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                let id = FeatureId(uuid.as_u128() as u64);

                let geometry_str: String = row.get("geometry");
                let geometry_json: serde_json::Value =
                    serde_json::from_str(&geometry_str).unwrap_or(serde_json::json!({}));
                let geometry = Geometry::from_geojson(&geometry_json);

                let properties: serde_json::Value = row.get("properties");
                let properties_map = properties
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                Feature {
                    id,
                    geometry,
                    properties: properties_map,
                    crs: filter.crs.epsg,
                }
            })
            .collect();

        Ok(features)
    }

    async fn get_feature(&self, id: FeatureId) -> Result<Option<Feature>> {
        let feature_uuid = Uuid::from_u128(id.0 as u128);

        let row = sqlx::query(
            r#"
            SELECT id, feature_id, ST_AsGeoJSON(geometry) as geometry, properties
            FROM features
            WHERE id = $1
            "#,
        )
        .bind(feature_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get feature: {}", e)))?;

        match row {
            Some(row) => {
                let geometry_str: String = row.get("geometry");
                let geometry_json: serde_json::Value = serde_json::from_str(&geometry_str)
                    .map_err(|e| {
                        GeoragError::Serialization(format!("Failed to parse geometry: {}", e))
                    })?;
                let geometry = Geometry::from_geojson(&geometry_json);

                let properties: serde_json::Value = row.get("properties");
                let properties_map = properties
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                Ok(Some(Feature {
                    id,
                    geometry,
                    properties: properties_map,
                    crs: 4326, // Default CRS
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_features_for_dataset(&self, dataset_id: DatasetId) -> Result<Vec<Feature>> {
        let dataset_uuid = Uuid::from_u128(dataset_id.0 as u128);

        let rows = sqlx::query(
            r#"
            SELECT id, feature_id, ST_AsGeoJSON(geometry) as geometry, properties
            FROM features
            WHERE dataset_id = $1
            "#,
        )
        .bind(dataset_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            GeoragError::Serialization(format!("Failed to get features for dataset: {}", e))
        })?;

        let features = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                let id = FeatureId(uuid.as_u128() as u64);

                let geometry_str: String = row.get("geometry");
                let geometry_json: serde_json::Value =
                    serde_json::from_str(&geometry_str).unwrap_or(serde_json::json!({}));
                let geometry = Geometry::from_geojson(&geometry_json);

                let properties: serde_json::Value = row.get("properties");
                let properties_map = properties
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                Feature {
                    id,
                    geometry,
                    properties: properties_map,
                    crs: 4326, // Default CRS
                }
            })
            .collect();

        Ok(features)
    }
}
