//! Integration tests for output formatting
//!
//! These tests verify that JSON output and dry-run mode work correctly.

use std::path::PathBuf;
use std::process::Command;

fn georag_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove 'deps' directory
    path.push("georag");
    path
}

#[test]
fn test_json_output_is_valid() {
    let test_dir = "/tmp/test-json-valid-unique";

    // Clean up if exists
    let _ = std::fs::remove_dir_all(test_dir);

    let output = Command::new(georag_bin())
        .args(["init", test_dir, "--json"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON to verify it's valid
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify structure
    assert!(parsed.get("status").is_some(), "Should have status field");
    assert!(parsed.get("data").is_some(), "Should have data field");

    // Clean up
    let _ = std::fs::remove_dir_all(test_dir);
}

#[test]
fn test_dry_run_no_state_modification() {
    let test_dir = "/tmp/test-dry-run-no-modify";

    // Clean up if exists
    let _ = std::fs::remove_dir_all(test_dir);

    // Run with dry-run
    let output = Command::new(georag_bin())
        .args(["init", test_dir, "--dry-run"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");

    // Verify no .georag directory was created
    let georag_dir = PathBuf::from(test_dir).join(".georag");
    assert!(!georag_dir.exists(), "Dry-run should not create .georag directory");
}

#[test]
fn test_dry_run_with_json_output() {
    let output = Command::new(georag_bin())
        .args(["init", "/tmp/test-dry-run-json", "--dry-run", "--json"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify dry-run structure
    let data = parsed.get("data").expect("Should have data field");
    assert_eq!(
        data.get("dry_run").and_then(|v| v.as_bool()),
        Some(true),
        "Should indicate dry_run mode"
    );
    assert!(data.get("planned_actions").is_some(), "Should have planned_actions field");
}

#[test]
fn test_actual_init_creates_files() {
    let test_dir = "/tmp/test-actual-init";

    // Clean up if exists
    let _ = std::fs::remove_dir_all(test_dir);

    // Run without dry-run
    let output = Command::new(georag_bin())
        .args(["init", test_dir])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");

    // Verify .georag directory was created
    let georag_dir = PathBuf::from(test_dir).join(".georag");
    assert!(georag_dir.exists(), "Should create .georag directory");
    assert!(georag_dir.join("config.toml").exists(), "Should create config.toml");
    assert!(georag_dir.join("datasets.json").exists(), "Should create datasets.json");

    // Clean up
    let _ = std::fs::remove_dir_all(test_dir);
}
