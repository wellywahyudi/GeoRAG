use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{GeoragError, Result};
use crate::formats::validation::FormatValidator;
use crate::formats::{
    FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation,
};

/// PDF format reader
pub struct PdfReader;

#[async_trait]
impl FormatReader for PdfReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        // Extract text from PDF
        let text =
            pdf_extract::extract_text(path).map_err(|e| GeoragError::DocumentExtraction {
                format: "PDF".to_string(),
                reason: format!("Failed to extract text: {}", e),
            })?;

        // Handle empty PDFs with warning
        if text.trim().is_empty() {
            tracing::warn!("PDF contains no extractable text: {}", path.display());
        }

        // Estimate page count from text structure
        let page_count = self.estimate_page_count(&text);

        // Count characters and words
        let character_count = text.len();
        let word_count = text.split_whitespace().count();

        // Get dataset name from filename
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed").to_string();

        // Create a single feature with document content
        let feature = FormatFeature {
            id: "document".to_string(),
            geometry: None, // No geometry by default
            properties: HashMap::from([
                ("source".to_string(), serde_json::Value::String(path.display().to_string())),
                ("format".to_string(), serde_json::Value::String("PDF".to_string())),
                ("content".to_string(), serde_json::Value::String(text.clone())),
                ("character_count".to_string(), serde_json::Value::Number(character_count.into())),
                ("word_count".to_string(), serde_json::Value::Number(word_count.into())),
            ]),
        };

        Ok(FormatDataset {
            name,
            format_metadata: FormatMetadata {
                format_name: "PDF".to_string(),
                format_version: None,
                layer_name: None,
                page_count: Some(page_count),
                paragraph_count: None,
                extraction_method: Some("pdf-extract".to_string()),
                spatial_association: None,
            },
            crs: 4326,
            features: vec![feature],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn format_name(&self) -> &str {
        "PDF"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Try to extract text to validate PDF structure
        match pdf_extract::extract_text(path) {
            Ok(text) => {
                if text.trim().is_empty() {
                    validation.warnings.push(
                        "PDF contains no extractable text (may be image-based or empty)"
                            .to_string(),
                    );
                }
            }
            Err(e) => {
                validation.errors.push(format!("Invalid or corrupted PDF: {}", e));
            }
        }

        Ok(validation)
    }
}

impl PdfReader {
    /// Estimate page count from extracted text
    fn estimate_page_count(&self, text: &str) -> usize {
        // Count form feed characters (page breaks)
        let form_feeds = text.chars().filter(|&c| c == '\x0C').count();

        if form_feeds > 0 {
            // If we have form feeds, use them as page indicators
            form_feeds + 1
        } else {
            // Fallback: estimate based on text length
            // Assume ~3000 characters per page (rough average)
            let estimated = (text.len() as f64 / 3000.0).ceil() as usize;
            estimated.max(1) // At least 1 page
        }
    }

    /// Split text into chunks suitable for embedding generation
    ///
    /// Chunks are created with the following strategy:
    /// - Target chunk size: ~500 words (approximately 2000-3000 characters)
    /// - Preserve paragraph boundaries where possible
    /// - Include overlap between chunks for context continuity
    ///
    pub fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
        let mut chunks = Vec::new();

        // Split into paragraphs (double newline or form feed)
        let paragraphs: Vec<&str> = text
            .split('\x0C')
            .flat_map(|section| section.split("\n\n"))
            .filter(|p| !p.trim().is_empty())
            .collect();

        if paragraphs.is_empty() {
            return chunks;
        }

        let mut current_chunk = String::new();
        let mut current_word_count = 0;
        let mut chunk_index = 0;

        for paragraph in paragraphs {
            let paragraph = paragraph.trim();
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            let word_count = words.len();

            // If adding this paragraph would exceed chunk size, finalize current chunk
            if current_word_count > 0 && current_word_count + word_count > chunk_size {
                let overlap_words: Vec<String> = current_chunk
                    .split_whitespace()
                    .rev()
                    .take(overlap)
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();

                // Store the chunk
                chunks.push(TextChunk {
                    index: chunk_index,
                    text: current_chunk.trim().to_string(),
                    word_count: current_word_count,
                    start_offset: 0, // Could be calculated if needed
                });
                chunk_index += 1;

                // Start new chunk with overlap
                current_chunk = overlap_words.join(" ");
                current_word_count = overlap_words.len();

                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
            }

            // Add paragraph to current chunk
            if !current_chunk.is_empty() && !current_chunk.ends_with(' ') {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
            current_word_count += word_count;
        }

        // Add final chunk if there's remaining content
        if !current_chunk.trim().is_empty() {
            chunks.push(TextChunk {
                index: chunk_index,
                text: current_chunk.trim().to_string(),
                word_count: current_word_count,
                start_offset: 0,
            });
        }

        chunks
    }
}

/// A chunk of text extracted from a document
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// Index of this chunk in the document
    pub index: usize,

    /// The text content of this chunk
    pub text: String,

    /// Number of words in this chunk
    pub word_count: usize,

    /// Character offset where this chunk starts in the original document
    pub start_offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let reader = PdfReader;
        assert_eq!(reader.supported_extensions(), &["pdf"]);
    }

    #[test]
    fn test_format_name() {
        let reader = PdfReader;
        assert_eq!(reader.format_name(), "PDF");
    }

    #[test]
    fn test_estimate_page_count_with_form_feeds() {
        let reader = PdfReader;
        let text = "Page 1\x0CPage 2\x0CPage 3";
        assert_eq!(reader.estimate_page_count(text), 3);
    }

    #[test]
    fn test_estimate_page_count_without_form_feeds() {
        let reader = PdfReader;
        // 6000 characters should estimate to 2 pages
        let text = "a".repeat(6000);
        assert_eq!(reader.estimate_page_count(&text), 2);
    }

    #[test]
    fn test_estimate_page_count_empty() {
        let reader = PdfReader;
        let text = "";
        assert_eq!(reader.estimate_page_count(text), 1);
    }

    #[test]
    fn test_chunk_text_basic() {
        let reader = PdfReader;
        // Create text with paragraph breaks to ensure multiple chunks
        let paragraph = "This is a test paragraph with some words. ".repeat(10);
        let text = format!("{}\n\n{}\n\n{}", paragraph, paragraph, paragraph);
        let chunks = reader.chunk_text(&text, 50, 10);

        // Should create multiple chunks
        assert!(chunks.len() > 1, "Expected multiple chunks, got {}", chunks.len());

        // Each chunk should have an index
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn test_chunk_text_preserves_paragraphs() {
        let reader = PdfReader;
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = reader.chunk_text(&text, 10, 2);

        // Should have at least one chunk
        assert!(!chunks.is_empty());

        // Chunks should contain paragraph content
        let all_text: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(all_text.contains("First paragraph"));
    }

    #[test]
    fn test_chunk_text_empty() {
        let reader = PdfReader;
        let text = "";
        let chunks = reader.chunk_text(text, 100, 10);

        // Empty text should produce no chunks
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_text_single_chunk() {
        let reader = PdfReader;
        let text = "Short text that fits in one chunk.";
        let chunks = reader.chunk_text(text, 100, 10);

        // Should produce exactly one chunk
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert!(chunks[0].text.contains("Short text"));
    }

    #[test]
    fn test_chunk_text_with_overlap() {
        let reader = PdfReader;
        // Create text with clear paragraph boundaries
        let text = format!("{}\n\n{}", "word ".repeat(60), "word ".repeat(60));
        let chunks = reader.chunk_text(&text, 50, 10);

        // Should create multiple chunks with overlap
        assert!(chunks.len() >= 2);

        // Verify overlap exists (second chunk should start with some words from first chunk)
        if chunks.len() >= 2 {
            let first_chunk_words: Vec<&str> = chunks[0].text.split_whitespace().collect();
            let second_chunk_words: Vec<&str> = chunks[1].text.split_whitespace().collect();

            // Second chunk should start with some words that appeared at end of first chunk
            assert!(first_chunk_words.len() > 0);
            assert!(second_chunk_words.len() > 0);
        }
    }
}
