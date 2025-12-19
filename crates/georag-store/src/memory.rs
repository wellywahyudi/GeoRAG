//! In-memory storage adapters for testing and development

use async_trait::async_trait;
use georag_core::error::Result;
use georag_core::models::query::{DistanceUnit, SpatialPredicate};
use georag_core::models::{
    ChunkId, Dataset, DatasetId, DatasetMeta, Embedding, Feature, FeatureId, ScoredResult,
    SpatialFilter, TextChunk,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::ports::{DocumentStore, SpatialStore, VectorStore};

/// In-memory implementation of SpatialStore
#[derive(Debug, Clone, Default)]
pub struct MemorySpatialStore {
    datasets: Arc<RwLock<HashMap<DatasetId, Dataset>>>,
    features: Arc<RwLock<HashMap<FeatureId, Feature>>>,
    next_id: Arc<RwLock<u64>>,
}

impl MemorySpatialStore {
    /// Create a new in-memory spatial store
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SpatialStore for MemorySpatialStore {
    async fn store_dataset(&self, dataset: &Dataset) -> Result<DatasetId> {
        let mut datasets = self.datasets.write().unwrap();
        let mut next_id = self.next_id.write().unwrap();

        let id = DatasetId(*next_id);
        *next_id += 1;

        let mut dataset_with_id = dataset.clone();
        dataset_with_id.id = id;

        datasets.insert(id, dataset_with_id);
        Ok(id)
    }

    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>> {
        let datasets = self.datasets.read().unwrap();
        Ok(datasets.get(&id).cloned())
    }

    async fn list_datasets(&self) -> Result<Vec<DatasetMeta>> {
        let datasets = self.datasets.read().unwrap();
        Ok(datasets
            .values()
            .map(|d| DatasetMeta {
                id: d.id,
                name: d.name.clone(),
                geometry_type: d.geometry_type,
                feature_count: d.feature_count,
                crs: d.crs,
                added_at: d.added_at,
            })
            .collect())
    }

    async fn delete_dataset(&self, id: DatasetId) -> Result<()> {
        let mut datasets = self.datasets.write().unwrap();
        datasets.remove(&id);
        Ok(())
    }

    async fn store_features(&self, features: &[Feature]) -> Result<()> {
        let mut store = self.features.write().unwrap();
        for feature in features {
            store.insert(feature.id.clone(), feature.clone());
        }
        Ok(())
    }

    async fn spatial_query(&self, filter: &SpatialFilter) -> Result<Vec<Feature>> {
        let features = self.features.read().unwrap();
        
        // Convert georag_core::models::SpatialFilter to georag_geo::models::SpatialFilter
        let _geo_filter = georag_geo::models::SpatialFilter {
            predicate: match filter.predicate {
                SpatialPredicate::Within => georag_geo::models::SpatialPredicate::Within,
                SpatialPredicate::Intersects => georag_geo::models::SpatialPredicate::Intersects,
                SpatialPredicate::Contains => georag_geo::models::SpatialPredicate::Contains,
                SpatialPredicate::BoundingBox => georag_geo::models::SpatialPredicate::BoundingBox,
            },
            geometry: filter.geometry.as_ref().and_then(|_g| {
                // Convert serde_json::Value to georag_geo::models::Geometry
                // For now, return None - this would need proper GeoJSON parsing
                None
            }),
            distance: filter.distance.map(|d| georag_geo::models::Distance {
                value: d.value,
                unit: match d.unit {
                    DistanceUnit::Meters => georag_geo::models::DistanceUnit::Meters,
                    DistanceUnit::Kilometers => georag_geo::models::DistanceUnit::Kilometers,
                    DistanceUnit::Miles => georag_geo::models::DistanceUnit::Miles,
                    DistanceUnit::Feet => georag_geo::models::DistanceUnit::Feet,
                },
            }),
            crs: georag_geo::models::Crs::new(filter.crs, ""),
        };

        Ok(features
            .values()
            .filter(|_feature| {
                // Convert feature geometry from serde_json::Value to georag_geo::models::Geometry
                // For now, skip spatial filtering - this would need proper GeoJSON parsing
                true
            })
            .cloned()
            .collect())
    }

    async fn get_feature(&self, id: FeatureId) -> Result<Option<Feature>> {
        let features = self.features.read().unwrap();
        Ok(features.get(&id).cloned())
    }
}

/// In-memory implementation of VectorStore
#[derive(Debug, Clone, Default)]
pub struct MemoryVectorStore {
    embeddings: Arc<RwLock<HashMap<ChunkId, Embedding>>>,
}

impl MemoryVectorStore {
    /// Create a new in-memory vector store
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn store_embeddings(&self, embeddings: &[Embedding]) -> Result<()> {
        let mut store = self.embeddings.write().unwrap();
        for embedding in embeddings {
            store.insert(embedding.chunk_id, embedding.clone());
        }
        Ok(())
    }

    async fn similarity_search(&self, query: &[f32], k: usize) -> Result<Vec<ScoredResult>> {
        let embeddings = self.embeddings.read().unwrap();

        let mut results: Vec<ScoredResult> = embeddings
            .values()
            .map(|embedding| {
                let score = Self::cosine_similarity(query, &embedding.vector);
                ScoredResult {
                    chunk_id: embedding.chunk_id,
                    score,
                    spatial_score: None,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        results.truncate(k);

        Ok(results)
    }

    async fn get_embedding(&self, chunk_id: ChunkId) -> Result<Option<Embedding>> {
        let embeddings = self.embeddings.read().unwrap();
        Ok(embeddings.get(&chunk_id).cloned())
    }

    async fn delete_embeddings(&self, chunk_ids: &[ChunkId]) -> Result<()> {
        let mut embeddings = self.embeddings.write().unwrap();
        for chunk_id in chunk_ids {
            embeddings.remove(chunk_id);
        }
        Ok(())
    }

    async fn dimensions(&self) -> Result<usize> {
        let embeddings = self.embeddings.read().unwrap();
        Ok(embeddings
            .values()
            .next()
            .map(|e| e.vector.len())
            .unwrap_or(0))
    }
}

/// In-memory implementation of DocumentStore
#[derive(Debug, Clone, Default)]
pub struct MemoryDocumentStore {
    chunks: Arc<RwLock<HashMap<ChunkId, TextChunk>>>,
}

impl MemoryDocumentStore {
    /// Create a new in-memory document store
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DocumentStore for MemoryDocumentStore {
    async fn store_chunks(&self, chunks: &[TextChunk]) -> Result<()> {
        let mut store = self.chunks.write().unwrap();
        for chunk in chunks {
            store.insert(chunk.id, chunk.clone());
        }
        Ok(())
    }

    async fn get_chunks(&self, ids: &[ChunkId]) -> Result<Vec<TextChunk>> {
        let chunks = self.chunks.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| chunks.get(id).cloned())
            .collect())
    }

    async fn get_chunk(&self, id: ChunkId) -> Result<Option<TextChunk>> {
        let chunks = self.chunks.read().unwrap();
        Ok(chunks.get(&id).cloned())
    }

    async fn delete_chunks(&self, ids: &[ChunkId]) -> Result<()> {
        let mut chunks = self.chunks.write().unwrap();
        for id in ids {
            chunks.remove(id);
        }
        Ok(())
    }

    async fn list_chunk_ids(&self) -> Result<Vec<ChunkId>> {
        let chunks = self.chunks.read().unwrap();
        Ok(chunks.keys().copied().collect())
    }
}
