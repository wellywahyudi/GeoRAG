use crate::output::OutputWriter;
use anyhow::{Context, Result};
use georag_core::formats::FormatRegistry;
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a file discovered during directory scanning
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path to the file
    pub path: PathBuf,

    /// Detected format name
    pub format_name: String,

    /// File size in bytes
    pub size: u64,
}

/// Result of processing a single file in a batch
#[derive(Debug, Clone)]
pub struct FileProcessingResult {
    pub path: PathBuf,
    pub format_name: String,
    pub error: Option<String>,
    pub dataset_name: Option<String>,
}

/// Summary of batch processing results
#[derive(Debug, Clone)]
pub struct BatchSummary {
    /// Total files discovered
    pub total_files: usize,

    /// Successfully processed files
    pub successful: Vec<FileProcessingResult>,

    /// Failed files
    pub failed: Vec<FileProcessingResult>,
}

impl BatchSummary {
    /// Create a new empty batch summary
    pub fn new() -> Self {
        Self {
            total_files: 0,
            successful: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Add a successful result
    pub fn add_success(&mut self, result: FileProcessingResult) {
        self.successful.push(result);
    }

    /// Add a failed result
    pub fn add_failure(&mut self, result: FileProcessingResult) {
        self.failed.push(result);
    }

    /// Get success count
    pub fn success_count(&self) -> usize {
        self.successful.len()
    }

    /// Get failure count
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }

    /// Check if all files succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }

    /// Get summary by format
    pub fn summary_by_format(&self) -> std::collections::HashMap<String, FormatSummary> {
        let mut format_summaries = std::collections::HashMap::new();

        // Count successful files by format
        for result in &self.successful {
            let summary = format_summaries
                .entry(result.format_name.clone())
                .or_insert_with(FormatSummary::new);
            summary.successful += 1;
        }

        // Count failed files by format
        for result in &self.failed {
            let summary = format_summaries
                .entry(result.format_name.clone())
                .or_insert_with(FormatSummary::new);
            summary.failed += 1;
        }

        format_summaries
    }

    /// Display summary to output
    pub fn display(&self, output: &OutputWriter) {
        output.section("Batch Processing Summary");
        output.kv("Total Files", self.total_files);
        output.kv("Successful", self.success_count());
        output.kv("Failed", self.failure_count());

        // Display summary by format
        let format_summaries = self.summary_by_format();
        if !format_summaries.is_empty() {
            output.section("Summary by Format");
            for (format_name, summary) in format_summaries.iter() {
                output.kv(
                    format_name,
                    format!("{} successful, {} failed", summary.successful, summary.failed),
                );
            }
        }

        if !self.successful.is_empty() {
            output.section("Successfully Processed");
            for result in &self.successful {
                output.info(format!(
                    "{} - {} ({})",
                    result.path.display(),
                    result.dataset_name.as_ref().unwrap_or(&"unknown".to_string()),
                    result.format_name
                ));
            }
        }

        if !self.failed.is_empty() {
            output.section("Failed Files");
            for result in &self.failed {
                output.error(format!(
                    "{} ({}) - {}",
                    result.path.display(),
                    result.format_name,
                    result.error.as_ref().unwrap_or(&"unknown error".to_string())
                ));
            }
        }
    }
}

/// Summary of processing results for a specific format
#[derive(Debug, Clone, Default)]
pub struct FormatSummary {
    /// Number of successful files
    pub successful: usize,

    /// Number of failed files
    pub failed: usize,
}

impl FormatSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Scan a directory for supported files
pub fn scan_directory(
    dir_path: &Path,
    registry: &FormatRegistry,
    recursive: bool,
) -> Result<Vec<DiscoveredFile>> {
    let mut discovered = Vec::new();

    // Get supported extensions from registry
    let supported_extensions: Vec<String> = registry.supported_formats();

    // Read directory entries
    let entries = fs::read_dir(dir_path)
        .context(format!("Failed to read directory: {}", dir_path.display()))?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Handle subdirectories if recursive
        if path.is_dir() && recursive {
            let sub_files = scan_directory(&path, registry, recursive)?;
            discovered.extend(sub_files);
            continue;
        }

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Check if file has supported extension
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            if supported_extensions.contains(&extension.to_string()) {
                // Get file metadata
                let metadata = fs::metadata(&path)
                    .context(format!("Failed to read file metadata: {}", path.display()))?;

                // Detect format
                if let Ok(reader) = registry.detect_format(&path) {
                    discovered.push(DiscoveredFile {
                        path: path.clone(),
                        format_name: reader.format_name().to_string(),
                        size: metadata.len(),
                    });
                }
            }
        }
    }

    Ok(discovered)
}

/// Display progress for a file being processed
pub fn display_file_progress(
    output: &OutputWriter,
    current: usize,
    total: usize,
    file: &DiscoveredFile,
) {
    output.info(format!(
        "[{}/{}] Processing {} ({}, {} bytes)",
        current,
        total,
        file.path.display(),
        file.format_name,
        file.size
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_summary_creation() {
        let summary = BatchSummary::new();
        assert_eq!(summary.total_files, 0);
        assert_eq!(summary.success_count(), 0);
        assert_eq!(summary.failure_count(), 0);
        assert!(summary.all_succeeded());
    }

    #[test]
    fn test_batch_summary_add_success() {
        let mut summary = BatchSummary::new();
        summary.add_success(FileProcessingResult {
            path: PathBuf::from("test.geojson"),
            format_name: "GeoJSON".to_string(),
            error: None,
            dataset_name: Some("test".to_string()),
        });

        assert_eq!(summary.success_count(), 1);
        assert_eq!(summary.failure_count(), 0);
        assert!(summary.all_succeeded());
    }

    #[test]
    fn test_batch_summary_add_failure() {
        let mut summary = BatchSummary::new();
        summary.add_failure(FileProcessingResult {
            path: PathBuf::from("test.geojson"),
            format_name: "GeoJSON".to_string(),
            error: Some("Invalid file".to_string()),
            dataset_name: None,
        });

        assert_eq!(summary.success_count(), 0);
        assert_eq!(summary.failure_count(), 1);
        assert!(!summary.all_succeeded());
    }

    #[test]
    fn test_batch_summary_by_format() {
        let mut summary = BatchSummary::new();

        // Add successful GeoJSON
        summary.add_success(FileProcessingResult {
            path: PathBuf::from("test1.geojson"),
            format_name: "GeoJSON".to_string(),
            error: None,
            dataset_name: Some("test1".to_string()),
        });

        // Add another successful GeoJSON
        summary.add_success(FileProcessingResult {
            path: PathBuf::from("test2.geojson"),
            format_name: "GeoJSON".to_string(),
            error: None,
            dataset_name: Some("test2".to_string()),
        });

        // Add failed Shapefile
        summary.add_failure(FileProcessingResult {
            path: PathBuf::from("test.shp"),
            format_name: "Shapefile".to_string(),
            error: Some("Missing .dbf file".to_string()),
            dataset_name: None,
        });

        // Add successful PDF
        summary.add_success(FileProcessingResult {
            path: PathBuf::from("doc.pdf"),
            format_name: "PDF".to_string(),
            error: None,
            dataset_name: Some("doc".to_string()),
        });

        let format_summaries = summary.summary_by_format();

        assert_eq!(format_summaries.len(), 3);
        assert_eq!(format_summaries.get("GeoJSON").unwrap().successful, 2);
        assert_eq!(format_summaries.get("GeoJSON").unwrap().failed, 0);
        assert_eq!(format_summaries.get("Shapefile").unwrap().successful, 0);
        assert_eq!(format_summaries.get("Shapefile").unwrap().failed, 1);
        assert_eq!(format_summaries.get("PDF").unwrap().successful, 1);
        assert_eq!(format_summaries.get("PDF").unwrap().failed, 0);
    }

    #[test]
    fn test_format_summary() {
        let summary = FormatSummary::new();
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
    }
}
