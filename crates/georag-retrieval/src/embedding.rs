use georag_core::error::Result;
use georag_core::models::{Embedding, FeatureId, Geometry, SpatialMetadata, TextChunk};
use georag_llm::ports::Embedder;
use std::sync::Arc;

/// Port trait for spatial store (re-exported from georag-store)
pub trait SpatialStore: Send + Sync {
    fn get_feature(
        &self,
        id: FeatureId,
    ) -> impl std::future::Future<Output = Result<Option<georag_core::models::Feature>>> + Send;
}

// Blanket implementation for Arc<dyn georag_store::ports::SpatialStore>
impl<T> SpatialStore for T
where
    T: georag_store::ports::SpatialStore,
{
    async fn get_feature(&self, id: FeatureId) -> Result<Option<georag_core::models::Feature>> {
        georag_store::ports::SpatialStore::get_feature(self, id).await
    }
}

/// Pipeline for generating embeddings from text chunks
pub struct EmbeddingPipeline<E: Embedder> {
    embedder: E,
    batch_size: usize,
}

impl<E: Embedder> EmbeddingPipeline<E> {
    /// Create a new embedding pipeline with the specified embedder and batch size
    pub fn new(embedder: E, batch_size: usize) -> Self {
        Self { embedder, batch_size }
    }

    /// Get the embedder's model name
    pub fn model_name(&self) -> &str {
        self.embedder.model_name()
    }

    /// Get the embedding dimension
    pub fn dimensions(&self) -> usize {
        self.embedder.dimensions()
    }

    /// Generate embeddings for all chunks with progress callback
    pub fn generate_embeddings<F>(
        &self,
        chunks: &[TextChunk],
        mut progress: F,
    ) -> Result<Vec<Embedding>>
    where
        F: FnMut(usize, usize),
    {
        let total = chunks.len();
        let mut all_embeddings = Vec::with_capacity(total);

        // Process chunks in batches
        for (batch_idx, chunk_batch) in chunks.chunks(self.batch_size).enumerate() {
            let texts: Vec<&str> = chunk_batch.iter().map(|c| c.content.as_str()).collect();

            // Generate embeddings for this batch
            let vectors = self.embedder.embed(&texts)?;

            // Create Embedding objects (without spatial metadata for now)
            for (chunk, vector) in chunk_batch.iter().zip(vectors.into_iter()) {
                let embedding = Embedding {
                    chunk_id: chunk.id,
                    vector,
                    spatial_metadata: None,
                };
                all_embeddings.push(embedding);
            }

            // Report progress
            let processed = (batch_idx + 1) * self.batch_size.min(chunk_batch.len());
            progress(processed.min(total), total);
        }

        Ok(all_embeddings)
    }

    /// Generate embeddings with spatial metadata attachment
    ///
    /// For chunks with spatial_ref, looks up the feature geometry and attaches spatial metadata.
    pub async fn generate_embeddings_with_spatial<F, S>(
        &self,
        chunks: &[TextChunk],
        spatial_store: Arc<S>,
        mut progress: F,
    ) -> Result<Vec<Embedding>>
    where
        F: FnMut(usize, usize),
        S: SpatialStore + ?Sized,
    {
        let total = chunks.len();
        let mut all_embeddings = Vec::with_capacity(total);

        // Process chunks in batches
        for (batch_idx, chunk_batch) in chunks.chunks(self.batch_size).enumerate() {
            let texts: Vec<&str> = chunk_batch.iter().map(|c| c.content.as_str()).collect();

            // Generate embeddings for this batch
            let vectors = self.embedder.embed(&texts)?;

            // Create Embedding objects with spatial metadata
            for (chunk, vector) in chunk_batch.iter().zip(vectors.into_iter()) {
                let spatial_metadata = if let Some(feature_id) = chunk.spatial_ref {
                    // Look up feature geometry
                    if let Some(feature) = spatial_store.get_feature(feature_id).await? {
                        // Extract bounding box from geometry
                        let bbox = extract_bbox(&feature.geometry);

                        Some(SpatialMetadata { feature_id, crs: feature.crs, bbox })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let embedding = Embedding {
                    chunk_id: chunk.id,
                    vector,
                    spatial_metadata,
                };
                all_embeddings.push(embedding);
            }

            // Report progress
            let processed = (batch_idx + 1) * self.batch_size.min(chunk_batch.len());
            progress(processed.min(total), total);
        }

        Ok(all_embeddings)
    }
}

/// Extract bounding box from typed Geometry
fn extract_bbox(geometry: &Option<Geometry>) -> Option<[f64; 4]> {
    let geom = geometry.as_ref()?;

    match geom {
        Geometry::Point { coordinates } => {
            Some([coordinates[0], coordinates[1], coordinates[0], coordinates[1]])
        }
        Geometry::LineString { coordinates } => compute_bbox_from_coords(coordinates),
        Geometry::MultiPoint { coordinates } => compute_bbox_from_coords(coordinates),
        Geometry::Polygon { coordinates } => {
            let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
            compute_bbox_from_coords(&all_coords)
        }
        Geometry::MultiLineString { coordinates } => {
            let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
            compute_bbox_from_coords(&all_coords)
        }
        Geometry::MultiPolygon { coordinates } => {
            let all_coords: Vec<[f64; 2]> = coordinates
                .iter()
                .flat_map(|poly| poly.iter().flatten())
                .cloned()
                .collect();
            compute_bbox_from_coords(&all_coords)
        }
    }
}

/// Compute bounding box from an array of coordinate pairs
fn compute_bbox_from_coords(coords: &[[f64; 2]]) -> Option<[f64; 4]> {
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
