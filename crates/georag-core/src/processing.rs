pub mod chunk;

use crate::error::{GeoragError, Result};
use crate::models::{ChunkId, ChunkMetadata, ChunkSource, FeatureId, TextChunk};
use std::collections::HashMap;

pub use chunk::ChunkGenerator;

#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Minimum chunk size in characters
    pub min_size: usize,

    /// Maximum chunk size in characters
    pub max_size: usize,

    /// Overlap size in characters for context preservation
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: 100,
            max_size: 1000,
            overlap: 50,
        }
    }
}

/// Chunk text into segments with configurable bounds and overlap
pub fn chunk_text(text: &str, config: &ChunkConfig, document_path: &str) -> Result<Vec<TextChunk>> {
    if config.min_size > config.max_size {
        return Err(GeoragError::ConfigInvalid {
            key: "chunk_size".to_string(),
            reason: format!(
                "min_size ({}) cannot be greater than max_size ({})",
                config.min_size, config.max_size
            ),
        });
    }

    if config.overlap >= config.max_size {
        return Err(GeoragError::ConfigInvalid {
            key: "chunk_overlap".to_string(),
            reason: format!(
                "overlap ({}) must be less than max_size ({})",
                config.overlap, config.max_size
            ),
        });
    }

    let mut chunks = Vec::new();
    let mut chunk_id = 0u64;
    let mut offset = 0;

    while offset < text.len() {
        let remaining = text.len() - offset;
        let chunk_size = if remaining <= config.max_size {
            remaining
        } else {
            let ideal_end = offset + config.max_size;
            find_break_point(text, offset, ideal_end, config.min_size)
        };

        let chunk_end = offset + chunk_size;
        let content = text[offset..chunk_end].to_string();

        let chunk = TextChunk {
            id: ChunkId(chunk_id),
            content,
            source: ChunkSource {
                document_path: document_path.to_string(),
                page: None,
                offset,
            },
            spatial_ref: None,
            metadata: ChunkMetadata {
                size: chunk_size,
                properties: HashMap::new(),
            },
        };

        chunks.push(chunk);
        chunk_id += 1;

        if chunk_end >= text.len() {
            break;
        }

        offset = chunk_end.saturating_sub(config.overlap);
    }

    Ok(chunks)
}

/// Find a good break point for chunking near the ideal position
fn find_break_point(text: &str, start: usize, ideal_end: usize, min_size: usize) -> usize {
    let max_end = ideal_end.min(text.len());
    let search_start = (start + min_size).max(start);
    let search_range = &text[search_start..max_end];

    if let Some(last_space) = search_range.rfind(|c: char| c.is_whitespace()) {
        return search_start - start + last_space + 1;
    }

    max_end - start
}

/// Associate a text chunk with a spatial feature
pub fn associate_chunk_with_geometry(mut chunk: TextChunk, feature_id: FeatureId) -> TextChunk {
    chunk.spatial_ref = Some(feature_id);
    chunk
}

/// Associate multiple chunks with a spatial feature
pub fn associate_chunks_with_geometry(
    chunks: Vec<TextChunk>,
    feature_id: FeatureId,
) -> Vec<TextChunk> {
    chunks
        .into_iter()
        .map(|chunk| associate_chunk_with_geometry(chunk, feature_id))
        .collect()
}

/// Chunk text and associate with geometry in one operation
pub fn chunk_text_with_geometry(
    text: &str,
    config: &ChunkConfig,
    document_path: &str,
    feature_id: FeatureId,
) -> Result<Vec<TextChunk>> {
    let chunks = chunk_text(text, config, document_path)?;
    Ok(associate_chunks_with_geometry(chunks, feature_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let text = "This is a test document with some text that needs to be chunked.";
        let config = ChunkConfig { min_size: 10, max_size: 30, overlap: 5 };

        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        assert!(!chunks.is_empty());

        // Verify all chunks are within bounds
        for chunk in &chunks {
            assert!(chunk.metadata.size >= config.min_size || chunk.metadata.size == text.len());
            assert!(chunk.metadata.size <= config.max_size);
        }

        // Verify no text is lost (reconstruct original minus overlaps)
        let mut reconstructed = String::new();
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                reconstructed.push_str(&chunk.content);
            } else {
                // Skip overlap from previous chunk
                let overlap_start = config.overlap.min(chunk.content.len());
                reconstructed.push_str(&chunk.content[overlap_start..]);
            }
        }

        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_chunk_text_short() {
        let text = "Short text.";
        let config = ChunkConfig::default();

        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn test_chunk_text_empty() {
        let text = "";
        let config = ChunkConfig::default();

        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_config_validation() {
        let text = "Some text";
        let invalid_config = ChunkConfig { min_size: 100, max_size: 50, overlap: 10 };

        let result = chunk_text(text, &invalid_config, "test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_overlap() {
        let text = "AAAA BBBB CCCC DDDD EEEE FFFF";
        let config = ChunkConfig { min_size: 5, max_size: 15, overlap: 5 };

        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        // Verify overlap exists between consecutive chunks
        for i in 1..chunks.len() {
            let prev_chunk = &chunks[i - 1];
            let curr_chunk = &chunks[i];

            // Current chunk should start before previous chunk ended
            assert!(curr_chunk.source.offset < prev_chunk.source.offset + prev_chunk.metadata.size);
        }
    }

    #[test]
    fn test_chunk_ids_sequential() {
        let text = "A".repeat(500);
        let config = ChunkConfig { min_size: 50, max_size: 100, overlap: 10 };

        let chunks = chunk_text(&text, &config, "test.txt").unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.id.0, i as u64);
        }
    }

    #[test]
    fn test_associate_chunk_with_geometry() {
        let text = "Test text";
        let config = ChunkConfig::default();
        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].spatial_ref.is_none());

        let feature_id = FeatureId(42);
        let associated = associate_chunk_with_geometry(chunks[0].clone(), feature_id);

        assert_eq!(associated.spatial_ref, Some(feature_id));
        assert_eq!(associated.content, chunks[0].content);
    }

    #[test]
    fn test_associate_chunks_with_geometry() {
        let text =
            "This is a longer text that will be split into multiple chunks for testing purposes.";
        let config = ChunkConfig { min_size: 10, max_size: 30, overlap: 5 };
        let chunks = chunk_text(text, &config, "test.txt").unwrap();

        assert!(chunks.len() > 1);

        // Verify none have spatial refs initially
        for chunk in &chunks {
            assert!(chunk.spatial_ref.is_none());
        }

        let feature_id = FeatureId(123);
        let associated = associate_chunks_with_geometry(chunks, feature_id);

        // Verify all have the same spatial ref
        for chunk in &associated {
            assert_eq!(chunk.spatial_ref, Some(feature_id));
        }
    }

    #[test]
    fn test_chunk_text_with_geometry() {
        let text = "Document text with associated geometry.";
        let config = ChunkConfig::default();
        let feature_id = FeatureId(999);

        let chunks = chunk_text_with_geometry(text, &config, "test.txt", feature_id).unwrap();

        assert!(!chunks.is_empty());

        // Verify all chunks have the spatial reference
        for chunk in &chunks {
            assert_eq!(chunk.spatial_ref, Some(feature_id));
            assert_eq!(chunk.source.document_path, "test.txt");
        }
    }

    #[test]
    fn test_chunk_source_tracking() {
        let text = "First chunk. Second chunk. Third chunk.";
        let config = ChunkConfig { min_size: 5, max_size: 20, overlap: 3 };

        let chunks = chunk_text(text, &config, "document.pdf").unwrap();

        // Verify ChunkSource is properly tracked
        for chunk in &chunks {
            assert_eq!(chunk.source.document_path, "document.pdf");
            assert!(chunk.source.offset < text.len());
            assert_eq!(chunk.source.page, None);
        }

        // Verify offsets are increasing
        for i in 1..chunks.len() {
            assert!(chunks[i].source.offset >= chunks[i - 1].source.offset);
        }
    }
}
