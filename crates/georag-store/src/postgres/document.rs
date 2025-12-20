//! DocumentStore implementation for PostgreSQL

use async_trait::async_trait;
use georag_core::error::{Result, GeoragError};
use georag_core::models::{ChunkId, TextChunk, FeatureId};
use sqlx::Row;
use uuid::Uuid;

use super::PostgresStore;
use crate::ports::DocumentStore;

#[async_trait]
impl DocumentStore for PostgresStore {
    async fn store_chunks(&self, chunks: &[TextChunk]) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        // Start a transaction for batch insert
        let mut tx = self.pool.begin().await
            .map_err(|e| GeoragError::Serialization(format!("Failed to begin transaction: {}", e)))?;

        // Get or create a default dataset and document for chunks
        // In a real implementation, chunks would be associated with a specific document
        // through the API call context or chunk metadata
        let dataset_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT id FROM datasets 
            WHERE name = 'default_documents'
            LIMIT 1
            "#
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to query dataset: {}", e)))?
        .unwrap_or_else(|| Uuid::new_v4());

        // If we generated a new UUID, we need to create the dataset
        let dataset_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM datasets WHERE id = $1)"
        )
        .bind(dataset_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to check dataset existence: {}", e)))?;

        if !dataset_exists {
            // Get or create default workspace
            let workspace_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO workspaces (name, crs, distance_unit, geometry_validity)
                VALUES ('default', 'EPSG:4326', 'Meters', 'Lenient')
                ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                RETURNING id
                "#
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to create workspace: {}", e)))?;

            // Create default dataset
            sqlx::query(
                r#"
                INSERT INTO datasets (id, workspace_id, name, source_path, format, crs, geometry_type, feature_count)
                VALUES ($1, $2, 'default_documents', '/tmp/default', 'text', 'EPSG:4326', 'GeometryCollection', 0)
                "#
            )
            .bind(dataset_id)
            .bind(workspace_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to create default dataset: {}", e)))?;
        }

        // Get or create a default document
        let document_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO documents (dataset_id, name, source_path, format)
            VALUES ($1, 'default_document', '/tmp/default', 'text')
            ON CONFLICT DO NOTHING
            RETURNING id
            "#
        )
        .bind(dataset_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to create document: {}", e)))?
        .unwrap_or_else(|| {
            // If conflict occurred, fetch the existing document
            Uuid::new_v4() // Placeholder, will be fetched below
        });

        // If we got a placeholder, fetch the actual document_id
        let document_id: Uuid = if document_id == Uuid::new_v4() {
            sqlx::query_scalar(
                "SELECT id FROM documents WHERE dataset_id = $1 AND name = 'default_document'"
            )
            .bind(dataset_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to fetch document: {}", e)))?
        } else {
            document_id
        };

        // Insert chunks
        for (idx, chunk) in chunks.iter().enumerate() {
            let chunk_uuid = Uuid::from_u128(chunk.id.0 as u128);

            // Convert metadata to JSONB
            let metadata_json = serde_json::to_value(&chunk.metadata)
                .map_err(|e| GeoragError::Serialization(format!("Failed to serialize metadata: {}", e)))?;

            // Handle spatial reference
            let spatial_ref_uuid = chunk.spatial_ref.map(|fid| Uuid::from_u128(fid.0 as u128));

            // For now, we'll use the chunk index from the loop if not available in metadata
            // In a real implementation, this would come from the chunk's source information
            let chunk_index = idx as i32;
            let start_offset = chunk.source.offset as i32;
            let end_offset = (chunk.source.offset + chunk.content.len()) as i32;

            sqlx::query(
                r#"
                INSERT INTO chunks (id, document_id, chunk_index, content, start_offset, end_offset, spatial_ref, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (document_id, chunk_index) DO UPDATE
                SET content = EXCLUDED.content,
                    start_offset = EXCLUDED.start_offset,
                    end_offset = EXCLUDED.end_offset,
                    spatial_ref = EXCLUDED.spatial_ref,
                    metadata = EXCLUDED.metadata
                "#
            )
            .bind(chunk_uuid)
            .bind(document_id)
            .bind(chunk_index)
            .bind(&chunk.content)
            .bind(start_offset)
            .bind(end_offset)
            .bind(spatial_ref_uuid)
            .bind(metadata_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to store chunk: {}", e)))?;
        }

        tx.commit().await
            .map_err(|e| GeoragError::Serialization(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    async fn get_chunks(&self, ids: &[ChunkId]) -> Result<Vec<TextChunk>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Convert ChunkIds to UUIDs
        let uuids: Vec<Uuid> = ids.iter()
            .map(|id| Uuid::from_u128(id.0 as u128))
            .collect();

        let rows = sqlx::query(
            r#"
            SELECT 
                c.id, 
                c.content, 
                c.start_offset, 
                c.end_offset,
                c.spatial_ref,
                c.metadata,
                d.source_path
            FROM chunks c
            JOIN documents d ON c.document_id = d.id
            WHERE c.id = ANY($1)
            "#
        )
        .bind(&uuids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get chunks: {}", e)))?;

        let chunks = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                let id = ChunkId(uuid.as_u128() as u64);

                let spatial_ref_uuid: Option<Uuid> = row.get("spatial_ref");
                let spatial_ref = spatial_ref_uuid.map(|uuid| FeatureId(uuid.as_u128() as u64));

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .unwrap_or_else(|_| georag_core::models::document::ChunkMetadata {
                        size: 0,
                        properties: std::collections::HashMap::new(),
                    });

                let document_path: String = row.get("source_path");
                let start_offset: i32 = row.get("start_offset");

                TextChunk {
                    id,
                    content: row.get("content"),
                    source: georag_core::models::document::ChunkSource {
                        document_path,
                        page: None,
                        offset: start_offset as usize,
                    },
                    spatial_ref,
                    metadata,
                }
            })
            .collect();

        Ok(chunks)
    }

    async fn get_chunk(&self, id: ChunkId) -> Result<Option<TextChunk>> {
        let chunks = self.get_chunks(&[id]).await?;
        Ok(chunks.into_iter().next())
    }

    async fn delete_chunks(&self, ids: &[ChunkId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        // Convert ChunkIds to UUIDs
        let uuids: Vec<Uuid> = ids.iter()
            .map(|id| Uuid::from_u128(id.0 as u128))
            .collect();

        // Delete chunks (CASCADE will handle embeddings)
        sqlx::query("DELETE FROM chunks WHERE id = ANY($1)")
            .bind(&uuids)
            .execute(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to delete chunks: {}", e)))?;

        Ok(())
    }

    async fn list_chunk_ids(&self) -> Result<Vec<ChunkId>> {
        let rows = sqlx::query("SELECT id FROM chunks ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to list chunk IDs: {}", e)))?;

        let ids = rows
            .into_iter()
            .map(|row| {
                let uuid: Uuid = row.get("id");
                ChunkId(uuid.as_u128() as u64)
            })
            .collect();

        Ok(ids)
    }
}
