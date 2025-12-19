//! Integration tests for layered configuration
//!
//! These tests verify that configuration loading follows the correct precedence:
//! CLI arguments > Environment variables > Config file > Defaults

use georag_core::config::{
    parse_distance_unit, parse_validity_mode, CliConfigOverrides, ConfigSource, LayeredConfig,
};
use georag_core::models::workspace::{DistanceUnit, ValidityMode};
use serial_test::serial;
use std::env;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_default_configuration() {
    let config = LayeredConfig::with_defaults();

    assert_eq!(config.crs.value, 4326);
    assert_eq!(config.crs.source, ConfigSource::Default);
    assert_eq!(config.distance_unit.value, DistanceUnit::Meters);
    assert_eq!(config.distance_unit.source, ConfigSource::Default);
    assert_eq!(config.geometry_validity.value, ValidityMode::Lenient);
    assert_eq!(config.embedder.value, "ollama:nomic-embed-text");
}

#[test]
fn test_file_overrides_defaults() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
crs = 3857
distance_unit = "Kilometers"
geometry_validity = "Strict"
embedder = "ollama:custom-model"
"#
    )
    .unwrap();

    let config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap();

    assert_eq!(config.crs.value, 3857);
    assert_eq!(config.crs.source, ConfigSource::File);
    assert_eq!(config.distance_unit.value, DistanceUnit::Kilometers);
    assert_eq!(config.distance_unit.source, ConfigSource::File);
    assert_eq!(config.geometry_validity.value, ValidityMode::Strict);
    assert_eq!(config.geometry_validity.source, ConfigSource::File);
    assert_eq!(config.embedder.value, "ollama:custom-model");
    assert_eq!(config.embedder.source, ConfigSource::File);
}

#[test]
fn test_partial_file_configuration() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
crs = 3857
# Only override CRS, leave others as defaults
"#
    )
    .unwrap();

    let config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap();

    assert_eq!(config.crs.value, 3857);
    assert_eq!(config.crs.source, ConfigSource::File);
    // These should still be defaults
    assert_eq!(config.distance_unit.value, DistanceUnit::Meters);
    assert_eq!(config.distance_unit.source, ConfigSource::Default);
    assert_eq!(config.embedder.source, ConfigSource::Default);
}

#[test]
#[serial]
fn test_environment_overrides_file() {
    // Clear any existing env vars first
    env::remove_var("GEORAG_CRS");
    env::remove_var("GEORAG_DISTANCE_UNIT");
    env::remove_var("GEORAG_EMBEDDER");
    
    // Set environment variables
    env::set_var("GEORAG_CRS", "32748");
    env::set_var("GEORAG_DISTANCE_UNIT", "miles");
    env::set_var("GEORAG_EMBEDDER", "ollama:env-model");

    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
crs = 3857
distance_unit = "Kilometers"
embedder = "ollama:file-model"
"#
    )
    .unwrap();

    let config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap()
        .load_from_env();

    // Environment should override file
    assert_eq!(config.crs.value, 32748);
    assert_eq!(config.crs.source, ConfigSource::Environment);
    assert_eq!(config.distance_unit.value, DistanceUnit::Miles);
    assert_eq!(config.distance_unit.source, ConfigSource::Environment);
    assert_eq!(config.embedder.value, "ollama:env-model");
    assert_eq!(config.embedder.source, ConfigSource::Environment);

    // Clean up
    env::remove_var("GEORAG_CRS");
    env::remove_var("GEORAG_DISTANCE_UNIT");
    env::remove_var("GEORAG_EMBEDDER");
}

#[test]
#[serial]
fn test_cli_overrides_all() {
    env::remove_var("GEORAG_CRS");
    env::set_var("GEORAG_CRS", "32748");

    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
crs = 3857
distance_unit = "Kilometers"
"#
    )
    .unwrap();

    let mut config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap()
        .load_from_env();

    // CLI should override everything
    let cli_overrides = CliConfigOverrides {
        crs: Some(4326),
        distance_unit: Some(DistanceUnit::Feet),
        geometry_validity: None,
        embedder: Some("ollama:cli-model".to_string()),
    };

    config.update_from_cli(cli_overrides);

    assert_eq!(config.crs.value, 4326);
    assert_eq!(config.crs.source, ConfigSource::Cli);
    assert_eq!(config.distance_unit.value, DistanceUnit::Feet);
    assert_eq!(config.distance_unit.source, ConfigSource::Cli);
    assert_eq!(config.embedder.value, "ollama:cli-model");
    assert_eq!(config.embedder.source, ConfigSource::Cli);

    // Clean up
    env::remove_var("GEORAG_CRS");
}

#[test]
#[serial]
fn test_configuration_precedence_order() {
    // Validates Configuration precedence
    
    // Clear any existing env vars first
    env::remove_var("GEORAG_CRS");
    
    env::set_var("GEORAG_CRS", "32748");

    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "crs = 3857").unwrap();

    let mut config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap()
        .load_from_env();

    // At this point, environment should have overridden file
    assert_eq!(config.crs.value, 32748);
    assert_eq!(config.crs.source, ConfigSource::Environment);

    // Now CLI should override environment
    config.update_from_cli(CliConfigOverrides {
        crs: Some(4326),
        ..Default::default()
    });

    assert_eq!(config.crs.value, 4326);
    assert_eq!(config.crs.source, ConfigSource::Cli);

    // Verify precedence levels
    assert!(ConfigSource::Cli.precedence() > ConfigSource::Environment.precedence());
    assert!(ConfigSource::Environment.precedence() > ConfigSource::File.precedence());
    assert!(ConfigSource::File.precedence() > ConfigSource::Default.precedence());

    env::remove_var("GEORAG_CRS");
}

#[test]
fn test_configuration_source_tracking() {
    // Configuration source is inspectable
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "crs = 3857\ndistance_unit = \"Kilometers\"").unwrap();

    let config = LayeredConfig::with_defaults()
        .load_from_file(file.path())
        .unwrap();

    let inspection_map = config.to_inspection_map();

    // Verify we can inspect the source of each value
    assert!(inspection_map.contains_key("crs"));
    assert!(inspection_map.contains_key("distance_unit"));
    assert!(inspection_map.contains_key("geometry_validity"));
    assert!(inspection_map.contains_key("embedder"));

    let (crs_value, crs_source) = &inspection_map["crs"];
    assert_eq!(crs_value, "EPSG:3857");
    assert_eq!(*crs_source, ConfigSource::File);

    let (embedder_value, embedder_source) = &inspection_map["embedder"];
    assert_eq!(embedder_value, "ollama:nomic-embed-text");
    assert_eq!(*embedder_source, ConfigSource::Default);
}

#[test]
fn test_parse_distance_unit_variations() {
    assert_eq!(
        parse_distance_unit("meters").unwrap(),
        DistanceUnit::Meters
    );
    assert_eq!(parse_distance_unit("m").unwrap(), DistanceUnit::Meters);
    assert_eq!(parse_distance_unit("M").unwrap(), DistanceUnit::Meters);
    assert_eq!(
        parse_distance_unit("METERS").unwrap(),
        DistanceUnit::Meters
    );

    assert_eq!(
        parse_distance_unit("kilometers").unwrap(),
        DistanceUnit::Kilometers
    );
    assert_eq!(
        parse_distance_unit("km").unwrap(),
        DistanceUnit::Kilometers
    );

    assert_eq!(parse_distance_unit("miles").unwrap(), DistanceUnit::Miles);
    assert_eq!(parse_distance_unit("mi").unwrap(), DistanceUnit::Miles);

    assert_eq!(parse_distance_unit("feet").unwrap(), DistanceUnit::Feet);
    assert_eq!(parse_distance_unit("ft").unwrap(), DistanceUnit::Feet);

    assert!(parse_distance_unit("invalid").is_err());
}

#[test]
fn test_parse_validity_mode_variations() {
    assert_eq!(
        parse_validity_mode("strict").unwrap(),
        ValidityMode::Strict
    );
    assert_eq!(
        parse_validity_mode("STRICT").unwrap(),
        ValidityMode::Strict
    );

    assert_eq!(
        parse_validity_mode("lenient").unwrap(),
        ValidityMode::Lenient
    );
    assert_eq!(
        parse_validity_mode("LENIENT").unwrap(),
        ValidityMode::Lenient
    );

    assert!(parse_validity_mode("invalid").is_err());
}

#[test]
fn test_invalid_toml_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "invalid toml content [[[").unwrap();

    let result = LayeredConfig::with_defaults().load_from_file(file.path());

    assert!(result.is_err());
}

#[test]
fn test_missing_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent = temp_dir.path().join("does_not_exist.toml");

    let result = LayeredConfig::with_defaults().load_from_file(&non_existent);

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_full_configuration_workflow() {
    // This test simulates a complete configuration workflow:
    // 1. Start with defaults
    // 2. Load from file
    // 3. Override with environment
    // 4. Override with CLI

    // Clear env vars first
    env::remove_var("GEORAG_DISTANCE_UNIT");
    env::remove_var("GEORAG_EMBEDDER");

    // Create a config file
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
crs = 3857
distance_unit = "Kilometers"
geometry_validity = "Strict"
embedder = "ollama:file-model"
"#,
    )
    .unwrap();

    // Set environment variables
    env::set_var("GEORAG_DISTANCE_UNIT", "miles");
    env::set_var("GEORAG_EMBEDDER", "ollama:env-model");

    // Load configuration
    let mut config = LayeredConfig::with_defaults()
        .load_from_file(&config_path)
        .unwrap()
        .load_from_env();

    // Verify state after file + env
    assert_eq!(config.crs.value, 3857); // From file
    assert_eq!(config.crs.source, ConfigSource::File);
    assert_eq!(config.distance_unit.value, DistanceUnit::Miles); // From env
    assert_eq!(config.distance_unit.source, ConfigSource::Environment);
    assert_eq!(config.geometry_validity.value, ValidityMode::Strict); // From file
    assert_eq!(config.embedder.value, "ollama:env-model"); // From env

    // Apply CLI overrides
    config.update_from_cli(CliConfigOverrides {
        crs: Some(4326),
        embedder: Some("ollama:cli-model".to_string()),
        ..Default::default()
    });

    // Verify final state
    assert_eq!(config.crs.value, 4326); // From CLI
    assert_eq!(config.crs.source, ConfigSource::Cli);
    assert_eq!(config.distance_unit.value, DistanceUnit::Miles); // Still from env
    assert_eq!(config.embedder.value, "ollama:cli-model"); // From CLI
    assert_eq!(config.embedder.source, ConfigSource::Cli);

    // Clean up
    env::remove_var("GEORAG_DISTANCE_UNIT");
    env::remove_var("GEORAG_EMBEDDER");
}
