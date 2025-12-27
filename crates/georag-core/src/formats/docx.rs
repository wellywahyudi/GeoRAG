//! DOCX format reader implementation
//!
//! This module provides text extraction from Microsoft Word DOCX documents using the docx-rs crate.
//! DOCX files are treated as documents without inherent geometry, but can be associated with
//! spatial locations through external metadata.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{GeoragError, Result};
use crate::formats::{FormatDataset, FormatFeature, FormatMetadata, FormatReader, FormatValidation};
use crate::formats::validation::FormatValidator;

/// DOCX format reader
pub struct DocxReader;

#[async_trait]
impl FormatReader for DocxReader {
    async fn read(&self, path: &Path) -> Result<FormatDataset> {
        // Read the DOCX file into memory
        let bytes = std::fs::read(path)
            .map_err(|e| GeoragError::DocumentExtraction {
                format: "DOCX".to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

        // Parse the DOCX document
        let docx = docx_rs::read_docx(&bytes)
            .map_err(|e| GeoragError::DocumentExtraction {
                format: "DOCX".to_string(),
                reason: format!("Failed to parse DOCX: {}", e),
            })?;

        // Extract text from paragraphs and tables
        let mut paragraphs = Vec::new();
        let mut full_text = String::new();
        let mut table_count = 0;

        for child in &docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(p) = child {
                let text = self.extract_paragraph_text(p);
                if !text.trim().is_empty() {
                    paragraphs.push(text.clone());
                    full_text.push_str(&text);
                    full_text.push_str("\n\n");
                }
            } else if let docx_rs::DocumentChild::Table(t) = child {
                // Extract table content
                let table_text = self.extract_table_text(t);
                if !table_text.trim().is_empty() {
                    table_count += 1;
                    paragraphs.push(table_text.clone());
                    full_text.push_str(&table_text);
                    full_text.push_str("\n\n");
                }
            }
        }

        // Handle empty documents with warning
        if full_text.trim().is_empty() {
            tracing::warn!("DOCX contains no extractable text: {}", path.display());
        }

        // Count words
        let word_count = full_text.split_whitespace().count();

        // Get dataset name from filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        // Create a single feature with document content
        let feature = FormatFeature {
            id: "document".to_string(),
            geometry: None, // No geometry by default
            properties: HashMap::from([
                ("source".to_string(), serde_json::Value::String(path.display().to_string())),
                ("format".to_string(), serde_json::Value::String("DOCX".to_string())),
                ("content".to_string(), serde_json::Value::String(full_text.clone())),
                ("word_count".to_string(), serde_json::Value::Number(word_count.into())),
                ("paragraph_count".to_string(), serde_json::Value::Number(paragraphs.len().into())),
                ("table_count".to_string(), serde_json::Value::Number(table_count.into())),
            ]),
        };

        Ok(FormatDataset {
            name,
            format_metadata: FormatMetadata {
                format_name: "DOCX".to_string(),
                format_version: None,
                layer_name: None,
                page_count: None,
                paragraph_count: Some(paragraphs.len()),
                extraction_method: Some("docx-rs".to_string()),
                spatial_association: None,
            },
            crs: 4326, // Default to WGS84 for documents
            features: vec![feature],
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["docx"]
    }

    fn format_name(&self) -> &str {
        "DOCX"
    }

    async fn validate(&self, path: &Path) -> Result<FormatValidation> {
        // Basic file validation
        let mut validation = FormatValidator::validate_file_exists(path);
        if !validation.is_valid() {
            return Ok(validation);
        }

        // Try to open and parse the DOCX to validate structure
        match std::fs::read(path) {
            Ok(bytes) => {
                match docx_rs::read_docx(&bytes) {
                    Ok(docx) => {
                        // Check if document has any content
                        let has_content = docx.document.children.iter().any(|child| {
                            matches!(child, docx_rs::DocumentChild::Paragraph(_) | docx_rs::DocumentChild::Table(_))
                        });
                        
                        if !has_content {
                            validation.warnings.push(
                                "DOCX appears to be empty (no paragraphs or tables found)".to_string()
                            );
                        }
                    }
                    Err(e) => {
                        validation.errors.push(format!("Invalid or corrupted DOCX: {}", e));
                    }
                }
            }
            Err(e) => {
                validation.errors.push(format!("Cannot open file: {}", e));
            }
        }

        Ok(validation)
    }
}

impl DocxReader {
    /// Extract text from a paragraph
    fn extract_paragraph_text(&self, paragraph: &docx_rs::Paragraph) -> String {
        paragraph.children
            .iter()
            .filter_map(|child| {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    Some(self.extract_run_text(run))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract text from a run
    fn extract_run_text(&self, run: &docx_rs::Run) -> String {
        run.children
            .iter()
            .filter_map(|child| {
                if let docx_rs::RunChild::Text(text) = child {
                    Some(text.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract text from a table
    fn extract_table_text(&self, table: &docx_rs::Table) -> String {
        let mut table_text = String::new();
        
        for row_child in &table.rows {
            let docx_rs::TableChild::TableRow(row) = row_child;
            let mut row_text = Vec::new();
            
            for cell_child in &row.cells {
                let docx_rs::TableRowChild::TableCell(cell) = cell_child;
                let cell_text = cell.children
                    .iter()
                    .filter_map(|child| {
                        if let docx_rs::TableCellContent::Paragraph(p) = child {
                            Some(self.extract_paragraph_text(p))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                
                if !cell_text.trim().is_empty() {
                    row_text.push(cell_text);
                }
            }
            
            if !row_text.is_empty() {
                table_text.push_str(&row_text.join(" | "));
                table_text.push('\n');
            }
        }
        
        table_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let reader = DocxReader;
        assert_eq!(reader.supported_extensions(), &["docx"]);
    }

    #[test]
    fn test_format_name() {
        let reader = DocxReader;
        assert_eq!(reader.format_name(), "DOCX");
    }

    // Note: Integration tests with actual DOCX files would require test fixtures
    // These would be added in the integration test suite
}
