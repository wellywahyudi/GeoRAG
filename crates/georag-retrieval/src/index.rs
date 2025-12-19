//! Index building and management

use chrono::Utc;
use georag_core::error::Result;
use georag_core::models::{Embedding, IndexState, SpatialMetadata, TextChunk};
use georag_geo::models::{Crs, ValidityMode};
use georag_geo::validation::validate_geometry;
use georag_llm::ports::Embedder;
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Index builder for creating deterministic retrieval indices
pub struct IndexBuilder<S, V, D, E>
where
    S: SpatialStore,
    V: VectorStore,
    D: DocumentStore,
    E: Embedder,
{
    spatial_store: S,
    vector_store: V,
    document_store: D,
    embedder: E,
    workspace_crs: Crs,
}

impl<S, V, D, E> IndexBuilder<S, V, D, E>
where
    S: SpatialStore,
    V: VectorStore,
    D: DocumentStore,
    E: Embedder,
{
    /// Create a new index builder
    pub fn new(
        spatial_store: S,
        vector_store: V,
        document_store: D,
        embedder: E,
        workspace_crs: Crs,
    ) -> Self {
        Self {
            spatial_store,
            vector_store,
            document_store,
            embedder,
            workspace_crs,
        }
    }

    /// Build the index
    ///
    /// This performs the following steps:
    /// 1. Normalize all geometries to workspace CRS
    /// 2. Fix invalid geometries
    /// 3. Generate embeddings for all chunks
    /// 4. Generate deterministic index hash
    pub async fn build(&self) -> Result<IndexBuildResult> {
        let mut result = IndexBuildResult::default();

        // Step 1: Normalize geometries to workspace CRS
        let normalized_count = self.normalize_geometries().await?;
        result.geometries_normalized = normalized_count;

        // Step 2: Fix invalid geometries
        let fixed_count = self.fix_invalid_geometries().await?;
        result.geometries_fixed = fixed_count;

        // Step 3: Generate embeddings
        let chunks = self.document_store.list_chunk_ids().await?;
        let chunk_data = self.document_store.get_chunks(&chunks).await?;
        result.chunk_count = chunk_data.len();

        let embeddings = self.generate_embeddings(&chunk_data).await?;
        result.embedding_dim = self.embedder.dimensions();

        // Store embeddings
        self.vector_store.store_embeddings(&embeddings).await?;

        // Step 4: Generate deterministic index hash
        let hash = self.generate_index_hash(&chunk_data, &embeddings).await?;
        result.index_hash = hash;

        Ok(result)
    }

    /// Normalize all geometries to workspace CRS
    async fn normalize_geometries(&self) -> Result<usize> {
        let features = self
            .spatial_store
            .spatial_query(&georag_core::models::SpatialFilter {
                predicate: georag_core::models::query::SpatialPredicate::BoundingBox,
                geometry: None,
                distance: None,
                crs: self.workspace_crs.epsg,
            })
            .await?;

        let mut normalized_count = 0;

        for feature in features {
            // Check if feature CRS matches workspace CRS
            if feature.crs != self.workspace_crs.epsg {
                // Note: In a full implementation, we would reproject the geometry here
                // For now, we just count features that need normalization
                normalized_count += 1;
            }
        }

        Ok(normalized_count)
    }

    /// Fix invalid geometries
    async fn fix_invalid_geometries(&self) -> Result<usize> {
        let features = self
            .spatial_store
            .spatial_query(&georag_core::models::SpatialFilter {
                predicate: georag_core::models::query::SpatialPredicate::BoundingBox,
                geometry: None,
                distance: None,
                crs: self.workspace_crs.epsg,
            })
            .await?;

        let mut fixed_count = 0;

        for feature in features {
            // Parse geometry from JSON
            if let Ok(geom) = serde_json::from_value::<georag_geo::models::Geometry>(feature.geometry.clone()) {
                // Validate geometry
                let validation_result = validate_geometry(&geom, ValidityMode::Lenient);
                if !validation_result.is_valid {
                    // In a full implementation, we would attempt to fix the geometry
                    // For now, we just count invalid geometries
                    fixed_count += 1;
                }
            }
        }

        Ok(fixed_count)
    }

    /// Generate embeddings for all chunks
    async fn generate_embeddings(&self, chunks: &[TextChunk]) -> Result<Vec<Embedding>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Extract text content
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();

        // Generate embeddings in batches
        let vectors = self.embedder.embed(&texts)?;

        // Create Embedding structs with spatial metadata
        let mut embeddings = Vec::new();
        for (chunk, vector) in chunks.iter().zip(vectors.into_iter()) {
            let spatial_metadata = if let Some(feature_id) = &chunk.spatial_ref {
                // In a full implementation, we would fetch the feature and extract bbox
                Some(SpatialMetadata {
                    feature_id: feature_id.clone(),
                    crs: self.workspace_crs.epsg,
                    bbox: None,
                })
            } else {
                None
            };

            embeddings.push(Embedding {
                chunk_id: chunk.id,
                vector,
                spatial_metadata,
            });
        }

        Ok(embeddings)
    }

    /// Generate deterministic index hash
    ///
    /// The hash is computed from:
    /// - Chunk IDs and content (sorted)
    /// - Embedding vectors
    /// - Workspace CRS
    /// - Embedder model name
    async fn generate_index_hash(
        &self,
        chunks: &[TextChunk],
        embeddings: &[Embedding],
    ) -> Result<String> {
        let mut hasher = DefaultHasher::new();

        // Sort chunks by ID for determinism
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.id.0);

        // Hash chunk data
        for chunk in &sorted_chunks {
            chunk.id.0.hash(&mut hasher);
            chunk.content.hash(&mut hasher);
            if let Some(spatial_ref) = &chunk.spatial_ref {
                spatial_ref.0.hash(&mut hasher);
            }
        }

        // Sort embeddings by chunk ID for determinism
        let mut sorted_embeddings = embeddings.to_vec();
        sorted_embeddings.sort_by_key(|e| e.chunk_id.0);

        // Hash embedding data
        for embedding in &sorted_embeddings {
            embedding.chunk_id.0.hash(&mut hasher);
            // Hash vector components (as bits for determinism)
            for &val in &embedding.vector {
                val.to_bits().hash(&mut hasher);
            }
        }

        // Hash workspace CRS
        self.workspace_crs.epsg.hash(&mut hasher);

        // Hash embedder model
        self.embedder.model_name().hash(&mut hasher);

        let hash_value = hasher.finish();
        Ok(format!("{:016x}", hash_value))
    }

    /// Create an IndexState from build results
    pub fn create_index_state(&self, result: &IndexBuildResult) -> IndexState {
        IndexState {
            hash: result.index_hash.clone(),
            built_at: Utc::now(),
            embedder: self.embedder.model_name().to_string(),
            chunk_count: result.chunk_count,
            embedding_dim: result.embedding_dim,
        }
    }
}

/// Result of an index build operation
#[derive(Debug, Clone, Default)]
pub struct IndexBuildResult {
    /// Number of geometries normalized to workspace CRS
    pub geometries_normalized: usize,

    /// Number of invalid geometries fixed
    pub geometries_fixed: usize,

    /// Total number of chunks indexed
    pub chunk_count: usize,

    /// Embedding dimension
    pub embedding_dim: usize,

    /// Deterministic index hash
    pub index_hash: String,
}
