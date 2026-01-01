use async_trait::async_trait;
use georag_core::error::{GeoragError, Result};
use georag_core::models::{ChunkId, Embedding, ScoredResult};
use sqlx::Row;
use uuid::Uuid;

use super::PostgresStore;
use crate::ports::VectorStore;

impl PostgresStore {
    /// Create IVFFlat index on embeddings table
    /// Lists parameter is auto-calculated if not provided in config
    pub async fn create_vector_index(&self) -> Result<()> {
        // Check if index already exists
        if self.vector_index_exists().await? {
            return Ok(());
        }

        // Calculate number of lists based on data size
        let lists = if let Some(lists) = self.config.indexes.ivfflat_lists {
            lists
        } else {
            // Auto-calculate: sqrt(row_count) is a common heuristic
            let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM embeddings")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    GeoragError::Serialization(format!("Failed to count embeddings: {}", e))
                })?;

            // Use sqrt(row_count) with min of 10 and max of 1000
            let calculated = (row_count as f64).sqrt() as usize;
            calculated.clamp(10, 1000)
        };

        // Create index with CONCURRENTLY if configured
        let create_sql = if self.config.indexes.rebuild_concurrently {
            format!(
                "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_embeddings_vector ON embeddings USING ivfflat(vector vector_cosine_ops) WITH (lists = {})",
                lists
            )
        } else {
            format!(
                "CREATE INDEX IF NOT EXISTS idx_embeddings_vector ON embeddings USING ivfflat(vector vector_cosine_ops) WITH (lists = {})",
                lists
            )
        };

        sqlx::query(&create_sql).execute(&self.pool).await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to create vector index: {}", e))
        })?;

        Ok(())
    }

    /// Check if vector index exists
    pub async fn vector_index_exists(&self) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE tablename = 'embeddings'
                AND indexname = 'idx_embeddings_vector'
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            GeoragError::Serialization(format!("Failed to check index existence: {}", e))
        })?;

        Ok(exists)
    }

    /// Drop and recreate vector index
    pub async fn rebuild_vector_index(&self) -> Result<()> {
        // Drop existing index if it exists
        sqlx::query("DROP INDEX IF EXISTS idx_embeddings_vector")
            .execute(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to drop index: {}", e)))?;

        // Create new index
        self.create_vector_index().await
    }
}

#[async_trait]
impl VectorStore for PostgresStore {
    async fn store_embeddings(&self, embeddings: &[Embedding]) -> Result<()> {
        if embeddings.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to begin transaction: {}", e))
        })?;

        for embedding in embeddings {
            let chunk_uuid = Uuid::from_u128(embedding.chunk_id.0 as u128);
            let embedding_uuid = Uuid::new_v4();

            // Convert Vec<f32> to pgvector format (as a string representation)
            let vector_str = format!(
                "[{}]",
                embedding.vector.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
            );

            let dimensions = embedding.vector.len() as i32;

            // Use ON CONFLICT for upsert behavior
            // We'll use a default model name if not specified
            let model_name = "default";

            sqlx::query(
                r#"
                INSERT INTO embeddings (id, chunk_id, model, dimensions, vector)
                VALUES ($1, $2, $3, $4, $5::vector)
                ON CONFLICT (chunk_id, model) DO UPDATE
                SET vector = EXCLUDED.vector,
                    dimensions = EXCLUDED.dimensions
                "#,
            )
            .bind(embedding_uuid)
            .bind(chunk_uuid)
            .bind(model_name)
            .bind(dimensions)
            .bind(vector_str)
            .execute(&mut *tx)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to store embedding: {}", e)))?;
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn similarity_search(
        &self,
        query: &[f32],
        k: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<ScoredResult>> {
        if !self.vector_index_exists().await? {
            eprintln!("Warning: Vector index does not exist. Falling back to exact search. Consider running create_vector_index() for better performance.");
        }

        // Convert query vector to pgvector format
        let query_str =
            format!("[{}]", query.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));

        // Build query with optional threshold filtering
        let query_sql = if let Some(_threshold) = threshold {
            r#"
                SELECT
                    e.chunk_id,
                    1 - (e.vector <=> $1::vector) as similarity
                FROM embeddings e
                WHERE 1 - (e.vector <=> $1::vector) >= $3
                ORDER BY e.vector <=> $1::vector
                LIMIT $2
                "#
            .to_string()
        } else {
            r#"
            SELECT
                e.chunk_id,
                1 - (e.vector <=> $1::vector) as similarity
            FROM embeddings e
            ORDER BY e.vector <=> $1::vector
            LIMIT $2
            "#
            .to_string()
        };

        let mut query_builder = sqlx::query(&query_sql).bind(&query_str).bind(k as i64);

        if let Some(threshold) = threshold {
            query_builder = query_builder.bind(threshold);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to execute similarity search: {}", e))
        })?;

        let results = rows
            .into_iter()
            .map(|row| {
                let chunk_uuid: Uuid = row.get("chunk_id");
                let chunk_id = ChunkId(chunk_uuid.as_u128() as u64);
                let similarity: f32 = row.get("similarity");

                ScoredResult {
                    chunk_id,
                    score: similarity,
                    spatial_score: None,
                }
            })
            .collect();

        Ok(results)
    }

    async fn get_embedding(&self, chunk_id: ChunkId) -> Result<Option<Embedding>> {
        let chunk_uuid = Uuid::from_u128(chunk_id.0 as u128);

        let row = sqlx::query(
            r#"
            SELECT chunk_id, vector::text as vector_text
            FROM embeddings
            WHERE chunk_id = $1
            LIMIT 1
            "#,
        )
        .bind(chunk_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get embedding: {}", e)))?;

        match row {
            Some(row) => {
                let vector_text: String = row.get("vector_text");

                // Parse pgvector format "[1.0,2.0,3.0]" to Vec<f32>
                let vector = parse_pgvector(&vector_text).map_err(|e| {
                    GeoragError::Serialization(format!("Failed to parse vector: {}", e))
                })?;

                Ok(Some(Embedding { chunk_id, vector, spatial_metadata: None }))
            }
            None => Ok(None),
        }
    }

    async fn delete_embeddings(&self, chunk_ids: &[ChunkId]) -> Result<()> {
        if chunk_ids.is_empty() {
            return Ok(());
        }

        // Convert ChunkIds to UUIDs
        let chunk_uuids: Vec<Uuid> =
            chunk_ids.iter().map(|id| Uuid::from_u128(id.0 as u128)).collect();

        // Batch DELETE
        sqlx::query("DELETE FROM embeddings WHERE chunk_id = ANY($1)")
            .bind(&chunk_uuids)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                GeoragError::Serialization(format!("Failed to delete embeddings: {}", e))
            })?;

        Ok(())
    }

    async fn dimensions(&self) -> Result<usize> {
        // Get dimensions from the first embedding
        let row = sqlx::query("SELECT dimensions FROM embeddings LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| GeoragError::Serialization(format!("Failed to get dimensions: {}", e)))?;

        match row {
            Some(row) => {
                let dims: i32 = row.get("dimensions");
                Ok(dims as usize)
            }
            None => Ok(0), // No embeddings stored yet
        }
    }
}

/// Parse pgvector format string "[1.0,2.0,3.0]" to Vec<f32>
fn parse_pgvector(s: &str) -> Result<Vec<f32>> {
    let trimmed = s.trim();

    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err(GeoragError::Serialization(format!("Invalid pgvector format: {}", s)));
    }

    let inner = &trimmed[1..trimmed.len() - 1];

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<f32>()
                .map_err(|e| GeoragError::Serialization(format!("Failed to parse float: {}", e)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pgvector() {
        assert_eq!(parse_pgvector("[1.0,2.0,3.0]").unwrap(), vec![1.0f32, 2.0, 3.0]);
        assert_eq!(parse_pgvector("[]").unwrap(), Vec::<f32>::new());
        assert_eq!(parse_pgvector("[1.5]").unwrap(), vec![1.5f32]);
        assert!(parse_pgvector("1.0,2.0").is_err());
        assert!(parse_pgvector("[1.0,invalid]").is_err());
    }
}
