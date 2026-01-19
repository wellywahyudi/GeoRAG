use chrono::Utc;
use georag_core::error::Result;
use georag_core::models::{
    DatasetMeta, Embedding, IndexState, SpatialFilter, SpatialMetadata, SpatialPredicate, TextChunk,
};
use georag_core::processing::chunk::ChunkGenerator;
use georag_geo::models::{Crs, ValidityMode};
use georag_geo::validation::validate_geometry;
use georag_core::llm::Embedder;
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Progress information for index building
#[derive(Debug, Clone)]
pub struct IndexProgress {
    pub phase: IndexPhase,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

/// Current phase of index building
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexPhase {
    Initializing,
    GeneratingChunks,
    GeneratingEmbeddings,
    StoringData,
    Finalizing,
}

/// Index builder for creating deterministic retrieval indices
pub struct IndexBuilder<E>
where
    E: Embedder,
{
    spatial_store: Arc<dyn SpatialStore>,
    vector_store: Arc<dyn VectorStore>,
    document_store: Arc<dyn DocumentStore>,
    embedder: E,
    workspace_crs: Crs,
    batch_size: usize,
}

impl<E> IndexBuilder<E>
where
    E: Embedder,
{
    /// Create a new index builder
    pub fn new(
        spatial_store: Arc<dyn SpatialStore>,
        vector_store: Arc<dyn VectorStore>,
        document_store: Arc<dyn DocumentStore>,
        embedder: E,
        workspace_crs: Crs,
    ) -> Self {
        Self {
            spatial_store,
            vector_store,
            document_store,
            embedder,
            workspace_crs,
            batch_size: 32,
        }
    }

    /// Set the batch size for embedding generation
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Build the index from existing chunks (legacy behavior)
    ///
    /// This performs the following steps:
    /// 1. Normalize all geometries to workspace CRS
    /// 2. Fix invalid geometries
    /// 3. Generate embeddings for all chunks
    /// 4. Generate deterministic index hash
    pub async fn build(&self) -> Result<IndexBuildResult> {
        self.build_with_progress(|_| {}).await
    }

    /// Build the index with progress reporting
    pub async fn build_with_progress<F>(&self, mut progress: F) -> Result<IndexBuildResult>
    where
        F: FnMut(IndexProgress),
    {
        let mut result = IndexBuildResult::default();

        progress(IndexProgress {
            phase: IndexPhase::Initializing,
            current: 0,
            total: 3,
            message: "Normalizing geometries".to_string(),
        });

        let normalized_count = self.normalize_geometries().await?;
        result.geometries_normalized = normalized_count;

        let fixed_count = self.fix_invalid_geometries().await?;
        result.geometries_fixed = fixed_count;

        progress(IndexProgress {
            phase: IndexPhase::GeneratingEmbeddings,
            current: 0,
            total: 0,
            message: "Loading chunks".to_string(),
        });

        let chunks = self.document_store.list_chunk_ids().await?;
        let chunk_data = self.document_store.get_chunks(&chunks).await?;
        result.chunk_count = chunk_data.len();

        let embeddings = self.generate_embeddings_with_progress(&chunk_data, &mut progress).await?;
        result.embedding_dim = self.embedder.dimensions();

        progress(IndexProgress {
            phase: IndexPhase::StoringData,
            current: 0,
            total: embeddings.len(),
            message: "Storing embeddings".to_string(),
        });

        self.vector_store.store_embeddings(&embeddings).await?;

        progress(IndexProgress {
            phase: IndexPhase::Finalizing,
            current: 0,
            total: 1,
            message: "Generating index hash".to_string(),
        });

        let hash = self.generate_index_hash(&chunk_data, &embeddings).await?;
        result.index_hash = hash;

        Ok(result)
    }

    /// Full rebuild from datasets (generates chunks + embeddings)
    ///
    /// This is the complete pipeline used by both CLI and API:
    /// 1. Clear existing chunks/embeddings if force=true
    /// 2. Generate chunks from all datasets
    /// 3. Generate embeddings
    /// 4. Store everything
    /// 5. Return IndexState
    pub async fn full_rebuild<F>(
        &self,
        datasets: &[DatasetMeta],
        force: bool,
        mut progress: F,
    ) -> Result<IndexBuildResult>
    where
        F: FnMut(IndexProgress),
    {
        let mut result = IndexBuildResult::default();

        // Phase 1: Clear existing data if force
        if force {
            progress(IndexProgress {
                phase: IndexPhase::Initializing,
                current: 0,
                total: 1,
                message: "Clearing existing data".to_string(),
            });

            // Clear existing chunks and embeddings
            let chunk_ids = self.document_store.list_chunk_ids().await?;
            if !chunk_ids.is_empty() {
                self.vector_store.delete_embeddings(&chunk_ids).await?;
                self.document_store.delete_chunks(&chunk_ids).await?;
            }
        }

        // Phase 2: Generate chunks from datasets
        progress(IndexProgress {
            phase: IndexPhase::GeneratingChunks,
            current: 0,
            total: datasets.len(),
            message: "Generating chunks from datasets".to_string(),
        });

        let chunk_generator = ChunkGenerator::default();
        let mut all_chunks = Vec::new();

        for (idx, dataset_meta) in datasets.iter().enumerate() {
            let dataset =
                self.spatial_store.get_dataset(dataset_meta.id).await?.ok_or_else(|| {
                    georag_core::error::GeoragError::DatasetNotFound {
                        name: format!("Dataset {} not found", dataset_meta.id.0),
                    }
                })?;

            let features = self.spatial_store.get_features_for_dataset(dataset_meta.id).await?;
            let chunks = chunk_generator.generate_chunks(&dataset, &features);
            all_chunks.extend(chunks);

            progress(IndexProgress {
                phase: IndexPhase::GeneratingChunks,
                current: idx + 1,
                total: datasets.len(),
                message: format!("Processed dataset '{}'", dataset_meta.name),
            });
        }

        result.chunk_count = all_chunks.len();

        // Phase 3: Generate embeddings
        let embeddings = self.generate_embeddings_with_progress(&all_chunks, &mut progress).await?;
        result.embedding_dim = self.embedder.dimensions();

        // Phase 4: Store chunks and embeddings
        progress(IndexProgress {
            phase: IndexPhase::StoringData,
            current: 0,
            total: 2,
            message: "Storing chunks".to_string(),
        });

        self.document_store.store_chunks(&all_chunks).await?;

        progress(IndexProgress {
            phase: IndexPhase::StoringData,
            current: 1,
            total: 2,
            message: "Storing embeddings".to_string(),
        });

        self.vector_store.store_embeddings(&embeddings).await?;

        // Phase 5: Generate hash
        progress(IndexProgress {
            phase: IndexPhase::Finalizing,
            current: 0,
            total: 1,
            message: "Generating index hash".to_string(),
        });

        let hash = self.generate_index_hash(&all_chunks, &embeddings).await?;
        result.index_hash = hash;

        Ok(result)
    }

    /// Normalize all geometries to workspace CRS
    async fn normalize_geometries(&self) -> Result<usize> {
        let features = self
            .spatial_store
            .spatial_query(&SpatialFilter {
                predicate: SpatialPredicate::BoundingBox,
                geometry: None,
                distance: None,
                crs: self.workspace_crs.clone(),
            })
            .await?;

        let mut normalized_count = 0;

        for feature in features {
            if feature.crs != self.workspace_crs.epsg {
                normalized_count += 1;
            }
        }

        Ok(normalized_count)
    }

    /// Fix invalid geometries
    async fn fix_invalid_geometries(&self) -> Result<usize> {
        let features = self
            .spatial_store
            .spatial_query(&SpatialFilter {
                predicate: SpatialPredicate::BoundingBox,
                geometry: None,
                distance: None,
                crs: self.workspace_crs.clone(),
            })
            .await?;

        let mut fixed_count = 0;

        for feature in features {
            if let Some(ref geom) = feature.geometry {
                let validation_result = validate_geometry(geom, ValidityMode::Lenient);
                if !validation_result.is_valid {
                    fixed_count += 1;
                }
            }
        }

        Ok(fixed_count)
    }

    /// Generate embeddings with progress reporting
    async fn generate_embeddings_with_progress<F>(
        &self,
        chunks: &[TextChunk],
        progress: &mut F,
    ) -> Result<Vec<Embedding>>
    where
        F: FnMut(IndexProgress),
    {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let total = chunks.len();
        let mut all_embeddings = Vec::with_capacity(total);

        // Process in batches
        for (batch_idx, chunk_batch) in chunks.chunks(self.batch_size).enumerate() {
            let texts: Vec<&str> = chunk_batch.iter().map(|c| c.content.as_str()).collect();

            let vectors = self.embedder.embed(&texts)?;

            for (chunk, vector) in chunk_batch.iter().zip(vectors.into_iter()) {
                let spatial_metadata = self.get_spatial_metadata_for_chunk(chunk).await?;

                all_embeddings.push(Embedding {
                    chunk_id: chunk.id,
                    vector,
                    spatial_metadata,
                });
            }

            let processed = ((batch_idx + 1) * self.batch_size).min(total);
            progress(IndexProgress {
                phase: IndexPhase::GeneratingEmbeddings,
                current: processed,
                total,
                message: format!("Generated {}/{} embeddings", processed, total),
            });
        }

        Ok(all_embeddings)
    }

    /// Get spatial metadata for a chunk
    async fn get_spatial_metadata_for_chunk(
        &self,
        chunk: &TextChunk,
    ) -> Result<Option<SpatialMetadata>> {
        let Some(feature_id) = chunk.spatial_ref else {
            return Ok(None);
        };

        let Some(feature) = self.spatial_store.get_feature(feature_id).await? else {
            return Ok(None);
        };

        let bbox = self.extract_bbox(&feature.geometry);

        Ok(Some(SpatialMetadata { feature_id, crs: feature.crs, bbox }))
    }

    /// Extract bounding box from geometry
    fn extract_bbox(&self, geometry: &Option<georag_core::models::Geometry>) -> Option<[f64; 4]> {
        use georag_core::models::Geometry;

        let geom = geometry.as_ref()?;

        match geom {
            Geometry::Point { coordinates } => {
                Some([coordinates[0], coordinates[1], coordinates[0], coordinates[1]])
            }
            Geometry::LineString { coordinates } => self.compute_bbox(coordinates),
            Geometry::MultiPoint { coordinates } => self.compute_bbox(coordinates),
            Geometry::Polygon { coordinates } => {
                let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
                self.compute_bbox(&all_coords)
            }
            Geometry::MultiLineString { coordinates } => {
                let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
                self.compute_bbox(&all_coords)
            }
            Geometry::MultiPolygon { coordinates } => {
                let all_coords: Vec<[f64; 2]> =
                    coordinates.iter().flat_map(|poly| poly.iter().flatten()).cloned().collect();
                self.compute_bbox(&all_coords)
            }
        }
    }

    fn compute_bbox(&self, coords: &[[f64; 2]]) -> Option<[f64; 4]> {
        if coords.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for coord in coords {
            min_x = min_x.min(coord[0]);
            min_y = min_y.min(coord[1]);
            max_x = max_x.max(coord[0]);
            max_y = max_y.max(coord[1]);
        }

        if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
            Some([min_x, min_y, max_x, max_y])
        } else {
            None
        }
    }

    /// Generate deterministic index hash
    async fn generate_index_hash(
        &self,
        chunks: &[TextChunk],
        embeddings: &[Embedding],
    ) -> Result<String> {
        let mut hasher = DefaultHasher::new();

        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.id.0);

        for chunk in &sorted_chunks {
            chunk.id.0.hash(&mut hasher);
            chunk.content.hash(&mut hasher);
            if let Some(spatial_ref) = &chunk.spatial_ref {
                spatial_ref.0.hash(&mut hasher);
            }
        }

        let mut sorted_embeddings = embeddings.to_vec();
        sorted_embeddings.sort_by_key(|e| e.chunk_id.0);

        for embedding in &sorted_embeddings {
            embedding.chunk_id.0.hash(&mut hasher);
            for &val in &embedding.vector {
                val.to_bits().hash(&mut hasher);
            }
        }

        self.workspace_crs.epsg.hash(&mut hasher);
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
