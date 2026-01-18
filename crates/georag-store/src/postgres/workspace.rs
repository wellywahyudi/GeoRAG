use async_trait::async_trait;
use georag_core::error::{GeoragError, Result};
use georag_core::models::workspace::{DistanceUnit, ValidityMode};
use georag_core::models::{
    DatasetId, DatasetMeta, GeometryType, WorkspaceConfig, WorkspaceId, WorkspaceMeta,
};
use sqlx::Row;
use uuid::Uuid;

use super::PostgresStore;
use crate::ports::WorkspaceStore;

#[async_trait]
impl WorkspaceStore for PostgresStore {
    async fn create_workspace(&self, name: &str, config: &WorkspaceConfig) -> Result<WorkspaceId> {
        let id = Uuid::new_v4();
        let crs_str = format!("EPSG:{}", config.crs);
        let distance_unit_str = format!("{:?}", config.distance_unit);
        let validity_str = format!("{:?}", config.geometry_validity);

        sqlx::query(
            r#"
            INSERT INTO workspaces (id, name, crs, distance_unit, geometry_validity)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(&crs_str)
        .bind(&distance_unit_str)
        .bind(&validity_str)
        .execute(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to create workspace: {}", e)))?;

        Ok(WorkspaceId(id))
    }

    async fn get_workspace(&self, id: WorkspaceId) -> Result<Option<WorkspaceMeta>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, crs, distance_unit, geometry_validity, created_at
            FROM workspaces
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get workspace: {}", e)))?;

        match row {
            Some(row) => {
                let uuid: Uuid = row.get("id");
                let crs_str: String = row.get("crs");
                let crs = crs_str
                    .strip_prefix("EPSG:")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(4326);

                let distance_unit_str: String = row.get("distance_unit");
                let distance_unit = match distance_unit_str.as_str() {
                    "Meters" => DistanceUnit::Meters,
                    "Kilometers" => DistanceUnit::Kilometers,
                    "Miles" => DistanceUnit::Miles,
                    "Feet" => DistanceUnit::Feet,
                    _ => DistanceUnit::Meters,
                };

                let validity_str: String = row.get("geometry_validity");
                let geometry_validity = match validity_str.as_str() {
                    "Strict" => ValidityMode::Strict,
                    "Lenient" => ValidityMode::Lenient,
                    _ => ValidityMode::Lenient,
                };

                Ok(Some(WorkspaceMeta {
                    id: WorkspaceId(uuid),
                    name: row.get("name"),
                    crs,
                    distance_unit,
                    geometry_validity,
                    created_at: row.get("created_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, crs, distance_unit, geometry_validity, created_at
            FROM workspaces
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to list workspaces: {}", e)))?;

        let workspaces = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                let crs_str: String = row.get("crs");
                let crs = crs_str
                    .strip_prefix("EPSG:")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(4326);

                let distance_unit_str: String = row.get("distance_unit");
                let distance_unit = match distance_unit_str.as_str() {
                    "Meters" => DistanceUnit::Meters,
                    "Kilometers" => DistanceUnit::Kilometers,
                    "Miles" => DistanceUnit::Miles,
                    "Feet" => DistanceUnit::Feet,
                    _ => DistanceUnit::Meters,
                };

                let validity_str: String = row.get("geometry_validity");
                let geometry_validity = match validity_str.as_str() {
                    "Strict" => ValidityMode::Strict,
                    "Lenient" => ValidityMode::Lenient,
                    _ => ValidityMode::Lenient,
                };

                WorkspaceMeta {
                    id: WorkspaceId(uuid),
                    name: row.get("name"),
                    crs,
                    distance_unit,
                    geometry_validity,
                    created_at: row.get("created_at"),
                }
            })
            .collect();

        Ok(workspaces)
    }

    async fn delete_workspace(&self, id: WorkspaceId) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                GeoragError::Serialization(format!("Failed to delete workspace: {}", e))
            })?;

        Ok(())
    }

    async fn list_datasets_for_workspace(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<DatasetMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, crs, geometry_type, feature_count, created_at
            FROM datasets
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            GeoragError::Serialization(format!("Failed to list datasets for workspace: {}", e))
        })?;

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

    async fn delete_dataset_in_workspace(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: DatasetId,
    ) -> Result<()> {
        let dataset_uuid = Uuid::from_u128(dataset_id.0 as u128);

        sqlx::query("DELETE FROM datasets WHERE id = $1 AND workspace_id = $2")
            .bind(dataset_uuid)
            .bind(workspace_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to delete dataset: {}", e)))?;

        Ok(())
    }
}
