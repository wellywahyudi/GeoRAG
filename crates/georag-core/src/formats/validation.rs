use crate::error::{GeoragError, Result};
use crate::formats::FormatValidation;
use std::path::Path;

pub struct FormatValidator;

impl FormatValidator {
    /// Validate that a file exists and is readable
    pub fn validate_file_exists(path: &Path) -> FormatValidation {
        let mut validation = FormatValidation::default();

        if !path.exists() {
            validation.errors.push(format!("File not found: {}", path.display()));
            return validation;
        }
        if let Err(e) = std::fs::metadata(path) {
            validation.errors.push(format!("Cannot access file: {}", e));
        }

        validation
    }

    /// Validate that a file has a specific extension
    pub fn validate_extension(path: &Path, expected_ext: &str) -> FormatValidation {
        let mut validation = FormatValidation::default();

        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case(expected_ext) => {}
            Some(ext) => {
                validation.errors.push(format!(
                    "Unexpected file extension: .{} (expected .{})",
                    ext, expected_ext
                ));
            }
            None => {
                validation
                    .errors
                    .push(format!("File has no extension (expected .{})", expected_ext));
            }
        }

        validation
    }

    /// Validate that required component files exist for multi-file formats
    pub fn validate_component_files(
        base_path: &Path,
        required_extensions: &[&str],
        optional_extensions: &[&str],
    ) -> FormatValidation {
        let mut validation = FormatValidation::default();

        for ext in required_extensions {
            let component_path = base_path.with_extension(ext);
            if !component_path.exists() {
                validation
                    .errors
                    .push(format!("Missing required file: {}", component_path.display()));
            }
        }

        for ext in optional_extensions {
            let component_path = base_path.with_extension(ext);
            if !component_path.exists() {
                validation.warnings.push(format!(
                    "Optional file not found: {} (may affect functionality)",
                    component_path.display()
                ));
            }
        }

        validation
    }

    /// Validate file size is within reasonable limits
    pub fn validate_file_size(path: &Path, max_size_mb: Option<u64>) -> FormatValidation {
        let mut validation = FormatValidation::default();

        match std::fs::metadata(path) {
            Ok(metadata) => {
                let size_mb = metadata.len() / (1024 * 1024);

                if let Some(max_size) = max_size_mb {
                    if size_mb > max_size {
                        validation.errors.push(format!(
                            "File size ({} MB) exceeds maximum allowed size ({} MB)",
                            size_mb, max_size
                        ));
                    } else if size_mb > max_size / 2 {
                        validation.warnings.push(format!(
                            "Large file ({} MB) may take longer to process",
                            size_mb
                        ));
                    }
                } else if size_mb > 100 {
                    // Warn for files over 100MB even without explicit limit
                    validation.warnings.push(format!(
                        "Very large file ({} MB) may take significant time to process",
                        size_mb
                    ));
                }
            }
            Err(e) => {
                validation.errors.push(format!("Cannot read file metadata: {}", e));
            }
        }

        validation
    }

    /// Validate that a text file is valid UTF-8
    pub fn validate_utf8(path: &Path) -> FormatValidation {
        let mut validation = FormatValidation::default();

        match std::fs::read_to_string(path) {
            Ok(_) => {}
            Err(e) => {
                validation
                    .errors
                    .push(format!("File is not valid UTF-8 or cannot be read: {}", e));
            }
        }

        validation
    }

    /// Validate XML structure by attempting to parse
    pub fn validate_xml_structure(path: &Path) -> FormatValidation {
        let mut validation = FormatValidation::default();

        match std::fs::read_to_string(path) {
            Ok(content) => {
                use quick_xml::Reader;
                let mut reader = Reader::from_str(&content);
                reader.config_mut().trim_text(true);

                let mut buf = Vec::new();
                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(quick_xml::events::Event::Eof) => break,
                        Err(e) => {
                            validation.errors.push(format!("Invalid XML structure: {}", e));
                            break;
                        }
                        _ => {}
                    }
                    buf.clear();
                }
            }
            Err(e) => {
                validation.errors.push(format!("Cannot read file: {}", e));
            }
        }

        validation
    }

    /// Validate JSON structure by attempting to parse
    pub fn validate_json_structure(path: &Path) -> FormatValidation {
        let mut validation = FormatValidation::default();

        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                    validation.errors.push(format!("Invalid JSON structure: {}", e));
                }
            }
            Err(e) => {
                validation.errors.push(format!("Cannot read file: {}", e));
            }
        }

        validation
    }

    /// Merge multiple validation results
    pub fn merge_validations(validations: Vec<FormatValidation>) -> FormatValidation {
        let mut merged = FormatValidation::default();

        for validation in validations {
            merged.errors.extend(validation.errors);
            merged.warnings.extend(validation.warnings);
        }

        merged
    }

    /// Convert a validation result to a Result type
    pub fn validation_to_result(validation: &FormatValidation, format_name: &str) -> Result<()> {
        if !validation.is_valid() {
            Err(GeoragError::FormatValidation {
                format: format_name.to_string(),
                reason: validation.errors.join("; "),
            })
        } else {
            Ok(())
        }
    }
}

/// Pre-read validation checks for common format issues
pub fn pre_read_validation(
    path: &Path,
    _format_name: &str,
    expected_extension: &str,
) -> FormatValidation {
    let validations = vec![
        FormatValidator::validate_file_exists(path),
        FormatValidator::validate_extension(path, expected_extension),
        FormatValidator::validate_file_size(path, None),
    ];

    FormatValidator::merge_validations(validations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_validate_file_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let existing_file = create_test_file(&temp_dir, "test.txt", "content");
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");

        // Existing file should pass
        let validation = FormatValidator::validate_file_exists(&existing_file);
        assert!(validation.is_valid());

        // Nonexistent file should fail
        let validation = FormatValidator::validate_file_exists(&nonexistent_file);
        assert!(!validation.is_valid());
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_validate_extension() {
        let path = Path::new("test.json");

        // Matching extension should pass
        let validation = FormatValidator::validate_extension(path, "json");
        assert!(validation.is_valid());

        // Non-matching extension should fail
        let validation = FormatValidator::validate_extension(path, "xml");
        assert!(!validation.is_valid());
    }

    #[test]
    fn test_validate_extension_case_insensitive() {
        let path = Path::new("test.JSON");

        // Should match case-insensitively
        let validation = FormatValidator::validate_extension(path, "json");
        assert!(validation.is_valid());
    }

    #[test]
    fn test_validate_component_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().join("test");

        // Create some component files
        create_test_file(&temp_dir, "test.shp", "");
        create_test_file(&temp_dir, "test.dbf", "");
        // Missing: test.shx

        let validation =
            FormatValidator::validate_component_files(&base_path, &["shp", "shx", "dbf"], &["prj"]);

        // Should have error for missing .shx
        assert!(!validation.is_valid());
        assert!(validation.errors.iter().any(|e| e.contains(".shx")));

        // Should have warning for missing .prj
        assert!(validation.has_warnings());
        assert!(validation.warnings.iter().any(|w| w.contains(".prj")));
    }

    #[test]
    fn test_validate_file_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let small_file = create_test_file(&temp_dir, "small.txt", "small content");

        // Small file should pass without warnings
        let validation = FormatValidator::validate_file_size(&small_file, Some(10));
        assert!(validation.is_valid());
        assert!(!validation.has_warnings());
    }

    #[test]
    fn test_validate_utf8() {
        let temp_dir = tempfile::tempdir().unwrap();
        let valid_file = create_test_file(&temp_dir, "valid.txt", "Hello, world!");

        // Valid UTF-8 should pass
        let validation = FormatValidator::validate_utf8(&valid_file);
        assert!(validation.is_valid());
    }

    #[test]
    fn test_validate_json_structure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let valid_json = create_test_file(&temp_dir, "valid.json", r#"{"key": "value"}"#);
        let invalid_json = create_test_file(&temp_dir, "invalid.json", "not json");

        // Valid JSON should pass
        let validation = FormatValidator::validate_json_structure(&valid_json);
        assert!(validation.is_valid());

        // Invalid JSON should fail
        let validation = FormatValidator::validate_json_structure(&invalid_json);
        assert!(!validation.is_valid());
    }

    #[test]
    fn test_validate_xml_structure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let valid_xml = create_test_file(&temp_dir, "valid.xml", "<root><child/></root>");
        let invalid_xml = create_test_file(&temp_dir, "invalid.xml", "<root><child></root>");

        // Valid XML should pass
        let validation = FormatValidator::validate_xml_structure(&valid_xml);
        assert!(validation.is_valid());

        // Invalid XML should fail (unclosed tag)
        let validation = FormatValidator::validate_xml_structure(&invalid_xml);
        assert!(!validation.is_valid());
    }

    #[test]
    fn test_merge_validations() {
        let mut val1 = FormatValidation::default();
        val1.errors.push("Error 1".to_string());
        val1.warnings.push("Warning 1".to_string());

        let mut val2 = FormatValidation::default();
        val2.errors.push("Error 2".to_string());
        val2.warnings.push("Warning 2".to_string());

        let merged = FormatValidator::merge_validations(vec![val1, val2]);

        assert_eq!(merged.errors.len(), 2);
        assert_eq!(merged.warnings.len(), 2);
        assert!(merged.errors.contains(&"Error 1".to_string()));
        assert!(merged.errors.contains(&"Error 2".to_string()));
    }

    #[test]
    fn test_validation_to_result() {
        let mut validation = FormatValidation::default();
        validation.errors.push("Test error".to_string());

        let result = FormatValidator::validation_to_result(&validation, "TestFormat");
        assert!(result.is_err());

        let validation = FormatValidation::default();
        let result = FormatValidator::validation_to_result(&validation, "TestFormat");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pre_read_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = create_test_file(&temp_dir, "test.json", "{}");

        let validation = pre_read_validation(&file, "JSON", "json");
        assert!(validation.is_valid());

        // Wrong extension
        let validation = pre_read_validation(&file, "JSON", "xml");
        assert!(!validation.is_valid());
    }
}
