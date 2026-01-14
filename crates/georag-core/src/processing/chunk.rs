use crate::error::{GeoragError, Result};
use crate::models::{
    ChunkId, ChunkMetadata, ChunkSource, Dataset, DatasetId, Feature, FeatureId, TextChunk,
};
use std::collections::HashMap;

/// Configuration for chunk generation
#[derive(Debug, Clone)]
pub struct ChunkGenerator {
    /// Minimum words per chunk
    pub min_chunk_size: usize,
    /// Maximum words per chunk
    pub max_chunk_size: usize,
    /// Word overlap between chunks
    pub overlap: usize,
}

impl Default for ChunkGenerator {
    fn default() -> Self {
        Self {
            min_chunk_size: 50,
            max_chunk_size: 500,
            overlap: 50,
        }
    }
}

impl ChunkGenerator {
    /// Create a new ChunkGenerator with custom configuration
    pub fn new(min_chunk_size: usize, max_chunk_size: usize, overlap: usize) -> Result<Self> {
        if min_chunk_size > max_chunk_size {
            return Err(GeoragError::ConfigInvalid {
                key: "chunk_size".to_string(),
                reason: format!(
                    "min_chunk_size ({}) cannot be greater than max_chunk_size ({})",
                    min_chunk_size, max_chunk_size
                ),
            });
        }

        if overlap >= max_chunk_size {
            return Err(GeoragError::ConfigInvalid {
                key: "chunk_overlap".to_string(),
                reason: format!(
                    "overlap ({}) must be less than max_chunk_size ({})",
                    overlap, max_chunk_size
                ),
            });
        }

        Ok(Self { min_chunk_size, max_chunk_size, overlap })
    }

    /// Generate chunks from a dataset's features
    pub fn generate_chunks(&self, dataset: &Dataset, features: &[Feature]) -> Vec<TextChunk> {
        let mut chunks = Vec::new();
        let mut global_chunk_index = 0u64;

        for feature in features {
            if let Some(text) = self.extract_text(feature) {
                let feature_chunks = self.chunk_text(
                    &text,
                    dataset.id,
                    feature.id,
                    &dataset.path.to_string_lossy(),
                    &mut global_chunk_index,
                );
                chunks.extend(feature_chunks);
            }
        }

        chunks
    }

    /// Extract text content from a feature following priority rules
    fn extract_text(&self, feature: &Feature) -> Option<String> {
        // Rule 1: If feature has "content" property, use it
        if let Some(content) = feature.properties.get("content") {
            if let Some(text) = content.as_str() {
                if !text.trim().is_empty() {
                    return Some(text.to_string());
                }
            }
        }

        // Rule 2: If feature has "name" and "description", concatenate them
        let name = feature
            .properties
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        let description = feature
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty());

        match (name, description) {
            (Some(n), Some(d)) => Some(format!("{}: {}", n, d)),
            // Rule 3: If only "name", use it
            (Some(n), None) => Some(n.to_string()),
            // Rule 4: If only "description", use it
            (None, Some(d)) => Some(d.to_string()),
            // Rule 5: No text to index
            (None, None) => None,
        }
    }

    /// Chunk text into segments with word-based boundaries
    fn chunk_text(
        &self,
        text: &str,
        dataset_id: DatasetId,
        feature_id: FeatureId,
        document_path: &str,
        global_chunk_index: &mut u64,
    ) -> Vec<TextChunk> {
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut word_offset = 0;

        while word_offset < words.len() {
            let remaining_words = words.len() - word_offset;

            // Determine chunk size in words
            let chunk_word_count = if remaining_words <= self.max_chunk_size {
                remaining_words
            } else {
                self.max_chunk_size
            };

            let chunk_words = &words[word_offset..word_offset + chunk_word_count];
            let content = chunk_words.join(" ");

            // Generate deterministic chunk ID
            let chunk_id = self.generate_chunk_id(dataset_id, feature_id, *global_chunk_index);

            let chunk = TextChunk {
                id: chunk_id,
                content: content.clone(),
                source: ChunkSource {
                    document_path: document_path.to_string(),
                    page: None,
                    offset: word_offset,
                },
                spatial_ref: Some(feature_id),
                metadata: ChunkMetadata {
                    size: content.len(),
                    properties: HashMap::new(),
                },
            };

            chunks.push(chunk);
            *global_chunk_index += 1;

            // Move to next chunk with overlap
            if word_offset + chunk_word_count >= words.len() {
                break;
            }

            word_offset += chunk_word_count.saturating_sub(self.overlap);
        }

        chunks
    }

    /// Generate deterministic ChunkId from dataset_id + feature_id + chunk_index
    fn generate_chunk_id(
        &self,
        dataset_id: DatasetId,
        feature_id: FeatureId,
        chunk_index: u64,
    ) -> ChunkId {
        // Combine the IDs using bit shifting to create a unique identifier
        // This ensures deterministic chunk IDs based on source
        let combined = (dataset_id.0 << 32) | (feature_id.0 << 16) | chunk_index;
        ChunkId(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_dataset() -> Dataset {
        Dataset {
            id: DatasetId(1),
            name: "test_dataset".to_string(),
            path: PathBuf::from("test.geojson"),
            geometry_type: crate::models::dataset::GeometryType::Point,
            feature_count: 1,
            crs: 4326,
            format: crate::models::dataset::FormatMetadata {
                format_name: "GeoJSON".to_string(),
                format_version: None,
                layer_name: None,
                page_count: None,
                paragraph_count: None,
                extraction_method: None,
                spatial_association: None,
            },
            added_at: chrono::Utc::now(),
        }
    }

    fn create_test_feature(id: u64, properties: HashMap<String, serde_json::Value>) -> Feature {
        use crate::models::Geometry;
        Feature {
            id: FeatureId(id),
            geometry: Some(Geometry::point(0.0, 0.0)),
            properties,
            crs: 4326,
        }
    }

    #[test]
    fn test_chunk_generator_default() {
        let generator = ChunkGenerator::default();
        assert_eq!(generator.min_chunk_size, 50);
        assert_eq!(generator.max_chunk_size, 500);
        assert_eq!(generator.overlap, 50);
    }

    #[test]
    fn test_chunk_generator_new_valid() {
        let generator = ChunkGenerator::new(10, 100, 10).unwrap();
        assert_eq!(generator.min_chunk_size, 10);
        assert_eq!(generator.max_chunk_size, 100);
        assert_eq!(generator.overlap, 10);
    }

    #[test]
    fn test_chunk_generator_new_invalid_size() {
        let result = ChunkGenerator::new(100, 50, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_generator_new_invalid_overlap() {
        let result = ChunkGenerator::new(50, 100, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_content_priority() {
        let generator = ChunkGenerator::default();

        let mut props = HashMap::new();
        props.insert("content".to_string(), serde_json::json!("Content text"));
        props.insert("name".to_string(), serde_json::json!("Name text"));
        props.insert("description".to_string(), serde_json::json!("Description text"));

        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, Some("Content text".to_string()));
    }

    #[test]
    fn test_extract_text_name_and_description() {
        let generator = ChunkGenerator::default();

        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("Park Name"));
        props.insert("description".to_string(), serde_json::json!("A beautiful park"));

        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, Some("Park Name: A beautiful park".to_string()));
    }

    #[test]
    fn test_extract_text_name_only() {
        let generator = ChunkGenerator::default();

        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("Location Name"));

        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, Some("Location Name".to_string()));
    }

    #[test]
    fn test_extract_text_description_only() {
        let generator = ChunkGenerator::default();

        let mut props = HashMap::new();
        props.insert("description".to_string(), serde_json::json!("Just a description"));

        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, Some("Just a description".to_string()));
    }

    #[test]
    fn test_extract_text_no_text() {
        let generator = ChunkGenerator::default();

        let props = HashMap::new();
        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, None);
    }

    #[test]
    fn test_extract_text_empty_strings() {
        let generator = ChunkGenerator::default();

        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("   "));
        props.insert("description".to_string(), serde_json::json!(""));

        let feature = create_test_feature(1, props);
        let text = generator.extract_text(&feature);

        assert_eq!(text, None);
    }

    #[test]
    fn test_generate_chunks_single_feature() {
        let generator = ChunkGenerator::new(5, 10, 2).unwrap();
        let dataset = create_test_dataset();

        let mut props = HashMap::new();
        props.insert(
            "content".to_string(),
            serde_json::json!("This is a test document with some text"),
        );
        let feature = create_test_feature(1, props);

        let chunks = generator.generate_chunks(&dataset, &[feature]);

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.spatial_ref.is_some());
            assert_eq!(chunk.spatial_ref.unwrap(), FeatureId(1));
        }
    }

    #[test]
    fn test_generate_chunks_multiple_features() {
        let generator = ChunkGenerator::default();
        let dataset = create_test_dataset();

        let mut props1 = HashMap::new();
        props1.insert("content".to_string(), serde_json::json!("Feature one text"));
        let feature1 = create_test_feature(1, props1);

        let mut props2 = HashMap::new();
        props2.insert("content".to_string(), serde_json::json!("Feature two text"));
        let feature2 = create_test_feature(2, props2);

        let chunks = generator.generate_chunks(&dataset, &[feature1, feature2]);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].spatial_ref, Some(FeatureId(1)));
        assert_eq!(chunks[1].spatial_ref, Some(FeatureId(2)));
    }

    #[test]
    fn test_generate_chunks_skips_features_without_text() {
        let generator = ChunkGenerator::default();
        let dataset = create_test_dataset();

        let props1 = HashMap::new();
        let feature1 = create_test_feature(1, props1);

        let mut props2 = HashMap::new();
        props2.insert("content".to_string(), serde_json::json!("Has text"));
        let feature2 = create_test_feature(2, props2);

        let chunks = generator.generate_chunks(&dataset, &[feature1, feature2]);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].spatial_ref, Some(FeatureId(2)));
    }

    #[test]
    fn test_chunk_id_deterministic() {
        let generator = ChunkGenerator::default();

        let id1 = generator.generate_chunk_id(DatasetId(1), FeatureId(2), 3);
        let id2 = generator.generate_chunk_id(DatasetId(1), FeatureId(2), 3);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_chunk_id_unique() {
        let generator = ChunkGenerator::default();

        let id1 = generator.generate_chunk_id(DatasetId(1), FeatureId(2), 3);
        let id2 = generator.generate_chunk_id(DatasetId(1), FeatureId(2), 4);
        let id3 = generator.generate_chunk_id(DatasetId(1), FeatureId(3), 3);
        let id4 = generator.generate_chunk_id(DatasetId(2), FeatureId(2), 3);

        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);
    }
}
