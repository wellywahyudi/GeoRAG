use crate::models::{ChunkId, Embedding, FeatureId, SpatialMetadata};

/// Create an embedding with spatial metadata
pub fn create_embedding_with_spatial_metadata(
    chunk_id: ChunkId,
    vector: Vec<f32>,
    feature_id: FeatureId,
    crs: u32,
    bbox: Option<[f64; 4]>,
) -> Embedding {
    Embedding {
        chunk_id,
        vector,
        spatial_metadata: Some(SpatialMetadata { feature_id, crs, bbox }),
    }
}

/// Create an embedding without spatial metadata
pub fn create_embedding(chunk_id: ChunkId, vector: Vec<f32>) -> Embedding {
    Embedding { chunk_id, vector, spatial_metadata: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_embedding_with_spatial_metadata() {
        let chunk_id = ChunkId(1);
        let vector = vec![0.1, 0.2, 0.3];
        let feature_id = FeatureId(123);
        let crs = 4326;
        let bbox = Some([-180.0, -90.0, 180.0, 90.0]);

        let embedding =
            create_embedding_with_spatial_metadata(chunk_id, vector.clone(), feature_id, crs, bbox);

        assert_eq!(embedding.chunk_id, chunk_id);
        assert_eq!(embedding.vector, vector);
        assert!(embedding.spatial_metadata.is_some());

        let metadata = embedding.spatial_metadata.unwrap();
        assert_eq!(metadata.feature_id, feature_id);
        assert_eq!(metadata.crs, crs);
        assert_eq!(metadata.bbox, bbox);
    }

    #[test]
    fn test_create_embedding_without_spatial_metadata() {
        let chunk_id = ChunkId(2);
        let vector = vec![0.4, 0.5, 0.6];

        let embedding = create_embedding(chunk_id, vector.clone());

        assert_eq!(embedding.chunk_id, chunk_id);
        assert_eq!(embedding.vector, vector);
        assert!(embedding.spatial_metadata.is_none());
    }
}
