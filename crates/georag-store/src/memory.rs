//! In-memory storage implementations for development and testing.
//!
//! These implementations use `RwLock::unwrap()` intentionally. Lock poisoning
//! only occurs when another thread panicked while holding the lock, which is
//! an unrecoverable state. For production workloads, use the PostgreSQL backend.

use async_trait::async_trait;
use georag_core::error::Result;
use georag_core::models::{
    ChunkId, Dataset, DatasetId, DatasetMeta, Embedding, Feature, FeatureId, ScoredResult,
    SpatialFilter, TextChunk,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::ports::{DocumentStore, SpatialStore, Transaction, Transactional, VectorStore};

/// In-memory implementation of SpatialStore
#[derive(Debug, Clone, Default)]
pub struct MemorySpatialStore {
    datasets: Arc<RwLock<HashMap<DatasetId, Dataset>>>,
    features: Arc<RwLock<HashMap<FeatureId, Feature>>>,
    dataset_features: Arc<RwLock<HashMap<DatasetId, Vec<FeatureId>>>>,
    next_id: Arc<RwLock<u64>>,
}

impl MemorySpatialStore {
    /// Create a new in-memory spatial store
    pub fn new() -> Self {
        Self::default()
    }

    /// Associate features with a dataset
    pub fn associate_features_with_dataset(
        &self,
        dataset_id: DatasetId,
        feature_ids: Vec<FeatureId>,
    ) {
        let mut dataset_features = self.dataset_features.write().unwrap();
        dataset_features.entry(dataset_id).or_default().extend(feature_ids);
    }

    /// Create a snapshot of the current state for transaction support
    fn create_snapshot(&self) -> MemoryStoreSnapshot {
        MemoryStoreSnapshot {
            datasets: self.datasets.read().unwrap().clone(),
            features: self.features.read().unwrap().clone(),
            dataset_features: self.dataset_features.read().unwrap().clone(),
            next_id: *self.next_id.read().unwrap(),
        }
    }

    /// Restore state from a snapshot (for rollback)
    fn restore_snapshot(&self, snapshot: MemoryStoreSnapshot) {
        *self.datasets.write().unwrap() = snapshot.datasets;
        *self.features.write().unwrap() = snapshot.features;
        *self.dataset_features.write().unwrap() = snapshot.dataset_features;
        *self.next_id.write().unwrap() = snapshot.next_id;
    }
}

/// Snapshot of MemorySpatialStore state for transaction rollback
#[derive(Clone)]
struct MemoryStoreSnapshot {
    datasets: HashMap<DatasetId, Dataset>,
    features: HashMap<FeatureId, Feature>,
    dataset_features: HashMap<DatasetId, Vec<FeatureId>>,
    next_id: u64,
}

/// Transaction for MemorySpatialStore
pub struct MemorySpatialTransaction {
    snapshot: MemoryStoreSnapshot,
    store: MemorySpatialStore,
    committed: bool,
}

#[async_trait]
impl Transaction for MemorySpatialTransaction {
    async fn commit(mut self: Box<Self>) -> Result<()> {
        self.committed = true;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<()> {
        if !self.committed {
            self.store.restore_snapshot(self.snapshot);
        }
        Ok(())
    }
}

#[async_trait]
impl Transactional for MemorySpatialStore {
    type Tx = MemorySpatialTransaction;

    async fn begin_transaction(&self) -> Result<Self::Tx> {
        Ok(MemorySpatialTransaction {
            snapshot: self.create_snapshot(),
            store: self.clone(),
            committed: false,
        })
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
            store.insert(feature.id, feature.clone());
        }
        Ok(())
    }

    async fn spatial_query(&self, filter: &SpatialFilter) -> Result<Vec<Feature>> {
        let features = self.features.read().unwrap();

        Ok(features
            .values()
            .filter(|feature| {
                // If no filter geometry, include all features
                if filter.geometry.is_none() {
                    return true;
                }

                // Get feature geometry
                let Some(ref feature_geom) = feature.geometry else {
                    return false; // No geometry, can't match spatial filter
                };

                // Apply spatial filter directly (types are now unified!)
                georag_geo::spatial::evaluate_spatial_filter(feature_geom, filter)
            })
            .cloned()
            .collect())
    }

    async fn get_feature(&self, id: FeatureId) -> Result<Option<Feature>> {
        let features = self.features.read().unwrap();
        Ok(features.get(&id).cloned())
    }

    async fn get_features_for_dataset(&self, dataset_id: DatasetId) -> Result<Vec<Feature>> {
        let dataset_features = self.dataset_features.read().unwrap();
        let features = self.features.read().unwrap();

        let feature_ids = dataset_features.get(&dataset_id);

        match feature_ids {
            Some(ids) => Ok(ids.iter().filter_map(|id| features.get(id).cloned()).collect()),
            None => Ok(Vec::new()),
        }
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

    async fn similarity_search(
        &self,
        query: &[f32],
        k: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<ScoredResult>> {
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

        // Apply threshold filtering if specified
        if let Some(threshold) = threshold {
            results.retain(|r| r.score >= threshold);
        }

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
        Ok(embeddings.values().next().map(|e| e.vector.len()).unwrap_or(0))
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
        Ok(ids.iter().filter_map(|id| chunks.get(id).cloned()).collect())
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_dataset(name: &str) -> Dataset {
        Dataset {
            id: DatasetId(0),
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{}.geojson", name)),
            geometry_type: georag_core::models::dataset::GeometryType::Point,
            feature_count: 0,
            crs: 4326,
            format: georag_core::models::dataset::FormatMetadata {
                format_name: "GeoJSON".to_string(),
                format_version: None,
                layer_name: None,
                page_count: None,
                paragraph_count: None,
                extraction_method: None,
                spatial_association: None,
            },
            added_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let store = MemorySpatialStore::new();

        // Begin transaction
        let tx = store.begin_transaction().await.unwrap();

        // Store dataset
        let dataset = create_test_dataset("test1");
        let id = store.store_dataset(&dataset).await.unwrap();

        // Commit
        Box::new(tx).commit().await.unwrap();

        // Dataset should still exist
        let retrieved = store.get_dataset(id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test1");
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let store = MemorySpatialStore::new();

        // Store initial dataset
        let dataset1 = create_test_dataset("before_tx");
        let id1 = store.store_dataset(&dataset1).await.unwrap();

        // Begin transaction
        let tx = store.begin_transaction().await.unwrap();

        // Store another dataset
        let dataset2 = create_test_dataset("during_tx");
        let id2 = store.store_dataset(&dataset2).await.unwrap();

        // Verify both exist before rollback
        assert!(store.get_dataset(id1).await.unwrap().is_some());
        assert!(store.get_dataset(id2).await.unwrap().is_some());

        // Rollback
        Box::new(tx).rollback().await.unwrap();

        // First dataset should still exist, second should be gone
        assert!(store.get_dataset(id1).await.unwrap().is_some());
        assert!(store.get_dataset(id2).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_transaction_rollback_restores_next_id() {
        let store = MemorySpatialStore::new();

        // Store initial dataset
        let dataset1 = create_test_dataset("first");
        store.store_dataset(&dataset1).await.unwrap();

        // Begin transaction
        let tx = store.begin_transaction().await.unwrap();

        // Store datasets in transaction
        store.store_dataset(&create_test_dataset("second")).await.unwrap();
        store.store_dataset(&create_test_dataset("third")).await.unwrap();

        // Rollback
        Box::new(tx).rollback().await.unwrap();

        // Next ID should be back to 1 (after first dataset)
        let next_dataset = create_test_dataset("after_rollback");
        let id = store.store_dataset(&next_dataset).await.unwrap();
        assert_eq!(id.0, 1); // Should be 1, not 3
    }
}
