use crate::error::{GeoragError, Result};
use crate::models::workspace::{DistanceUnit, ValidityMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Configuration source for tracking where values come from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigSource {
    /// Default value
    Default,
    /// Loaded from config file
    File,
    /// Loaded from environment variable
    Environment,
    /// Provided via CLI argument
    Cli,
}

impl ConfigSource {
    /// Returns the precedence level (higher = higher priority)
    pub fn precedence(&self) -> u8 {
        match self {
            ConfigSource::Default => 0,
            ConfigSource::File => 1,
            ConfigSource::Environment => 2,
            ConfigSource::Cli => 3,
        }
    }
}

/// A configuration value with its source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue<T> {
    pub value: T,
    pub source: ConfigSource,
}

impl<T> ConfigValue<T> {
    pub fn new(value: T, source: ConfigSource) -> Self {
        Self { value, source }
    }

    /// Update the value if the new source has higher precedence
    pub fn update(&mut self, value: T, source: ConfigSource) {
        if source.precedence() > self.source.precedence() {
            self.value = value;
            self.source = source;
        }
    }
}

/// Layered configuration for GeoRAG
#[derive(Debug, Clone)]
pub struct LayeredConfig {
    pub crs: ConfigValue<u32>,
    pub distance_unit: ConfigValue<DistanceUnit>,
    pub geometry_validity: ConfigValue<ValidityMode>,
    pub embedder: ConfigValue<String>,
}

impl LayeredConfig {
    /// Create a new configuration with default values
    pub fn with_defaults() -> Self {
        Self {
            crs: ConfigValue::new(4326, ConfigSource::Default),
            distance_unit: ConfigValue::new(DistanceUnit::Meters, ConfigSource::Default),
            geometry_validity: ConfigValue::new(ValidityMode::Lenient, ConfigSource::Default),
            embedder: ConfigValue::new(
                "ollama:nomic-embed-text".to_string(),
                ConfigSource::Default,
            ),
        }
    }

    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|e| GeoragError::ConfigInvalid {
                key: "file".to_string(),
                reason: format!("Failed to read config file: {}", e),
            })?;

        let file_config: FileConfig =
            toml::from_str(&content).map_err(|e| GeoragError::ConfigInvalid {
                key: "file".to_string(),
                reason: format!("Failed to parse TOML: {}", e),
            })?;

        // Update values from file
        if let Some(crs) = file_config.crs {
            self.crs.update(crs, ConfigSource::File);
        }

        if let Some(distance_unit) = file_config.distance_unit {
            self.distance_unit.update(distance_unit, ConfigSource::File);
        }

        if let Some(geometry_validity) = file_config.geometry_validity {
            self.geometry_validity.update(geometry_validity, ConfigSource::File);
        }

        if let Some(embedder) = file_config.embedder {
            self.embedder.update(embedder, ConfigSource::File);
        }

        Ok(self)
    }

    /// Load configuration from environment variables
    pub fn load_from_env(mut self) -> Self {
        // GEORAG_CRS
        if let Ok(crs_str) = env::var("GEORAG_CRS") {
            match crs_str.parse::<u32>() {
                Ok(crs) => self.crs.update(crs, ConfigSource::Environment),
                Err(_) => tracing::warn!(
                    "Invalid GEORAG_CRS value '{}': expected integer EPSG code",
                    crs_str
                ),
            }
        }

        // GEORAG_DISTANCE_UNIT
        if let Ok(unit_str) = env::var("GEORAG_DISTANCE_UNIT") {
            match parse_distance_unit(&unit_str) {
                Ok(unit) => self.distance_unit.update(unit, ConfigSource::Environment),
                Err(_) => tracing::warn!(
                    "Invalid GEORAG_DISTANCE_UNIT value '{}': expected meters, kilometers, miles, or feet",
                    unit_str
                ),
            }
        }

        // GEORAG_GEOMETRY_VALIDITY
        if let Ok(validity_str) = env::var("GEORAG_GEOMETRY_VALIDITY") {
            match parse_validity_mode(&validity_str) {
                Ok(validity) => self.geometry_validity.update(validity, ConfigSource::Environment),
                Err(_) => tracing::warn!(
                    "Invalid GEORAG_GEOMETRY_VALIDITY value '{}': expected strict or lenient",
                    validity_str
                ),
            }
        }

        // GEORAG_EMBEDDER
        if let Ok(embedder) = env::var("GEORAG_EMBEDDER") {
            self.embedder.update(embedder, ConfigSource::Environment);
        }

        self
    }

    /// Update configuration from CLI arguments
    pub fn update_from_cli(&mut self, overrides: CliConfigOverrides) {
        if let Some(crs) = overrides.crs {
            self.crs.update(crs, ConfigSource::Cli);
        }

        if let Some(distance_unit) = overrides.distance_unit {
            self.distance_unit.update(distance_unit, ConfigSource::Cli);
        }

        if let Some(geometry_validity) = overrides.geometry_validity {
            self.geometry_validity.update(geometry_validity, ConfigSource::Cli);
        }

        if let Some(embedder) = overrides.embedder {
            self.embedder.update(embedder, ConfigSource::Cli);
        }
    }

    /// Get all configuration values as a map for inspection
    pub fn to_inspection_map(&self) -> HashMap<String, (String, ConfigSource)> {
        let mut map = HashMap::new();

        map.insert("crs".to_string(), (format!("EPSG:{}", self.crs.value), self.crs.source));

        map.insert(
            "distance_unit".to_string(),
            (format!("{:?}", self.distance_unit.value), self.distance_unit.source),
        );

        map.insert(
            "geometry_validity".to_string(),
            (format!("{:?}", self.geometry_validity.value), self.geometry_validity.source),
        );

        map.insert("embedder".to_string(), (self.embedder.value.clone(), self.embedder.source));

        map
    }
}

/// Configuration loaded from TOML file
#[derive(Debug, Deserialize, Serialize)]
struct FileConfig {
    crs: Option<u32>,
    distance_unit: Option<DistanceUnit>,
    geometry_validity: Option<ValidityMode>,
    embedder: Option<String>,
}

/// CLI configuration overrides
#[derive(Debug, Default)]
pub struct CliConfigOverrides {
    pub crs: Option<u32>,
    pub distance_unit: Option<DistanceUnit>,
    pub geometry_validity: Option<ValidityMode>,
    pub embedder: Option<String>,
}

/// Parse distance unit from string
pub fn parse_distance_unit(s: &str) -> Result<DistanceUnit> {
    match s.to_lowercase().as_str() {
        "meters" | "m" => Ok(DistanceUnit::Meters),
        "kilometers" | "km" => Ok(DistanceUnit::Kilometers),
        "miles" | "mi" => Ok(DistanceUnit::Miles),
        "feet" | "ft" => Ok(DistanceUnit::Feet),
        _ => Err(GeoragError::ConfigInvalid {
            key: "distance_unit".to_string(),
            reason: format!("Invalid distance unit: {}. Use meters, kilometers, miles, or feet", s),
        }),
    }
}

/// Parse validity mode from string
pub fn parse_validity_mode(s: &str) -> Result<ValidityMode> {
    match s.to_lowercase().as_str() {
        "strict" => Ok(ValidityMode::Strict),
        "lenient" => Ok(ValidityMode::Lenient),
        _ => Err(GeoragError::ConfigInvalid {
            key: "geometry_validity".to_string(),
            reason: format!("Invalid validity mode: {}. Use strict or lenient", s),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = LayeredConfig::with_defaults();
        assert_eq!(config.crs.value, 4326);
        assert_eq!(config.crs.source, ConfigSource::Default);
        assert_eq!(config.distance_unit.value, DistanceUnit::Meters);
        assert_eq!(config.embedder.value, "ollama:nomic-embed-text");
    }

    #[test]
    fn test_config_precedence() {
        let mut value = ConfigValue::new(100, ConfigSource::Default);

        // File should override default
        value.update(200, ConfigSource::File);
        assert_eq!(value.value, 200);
        assert_eq!(value.source, ConfigSource::File);

        // Environment should override file
        value.update(300, ConfigSource::Environment);
        assert_eq!(value.value, 300);
        assert_eq!(value.source, ConfigSource::Environment);

        // CLI should override environment
        value.update(400, ConfigSource::Cli);
        assert_eq!(value.value, 400);
        assert_eq!(value.source, ConfigSource::Cli);

        // Lower precedence should not override
        value.update(500, ConfigSource::File);
        assert_eq!(value.value, 400); // Still CLI value
        assert_eq!(value.source, ConfigSource::Cli);
    }

    #[test]
    fn test_load_from_file() {
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

        let config = LayeredConfig::with_defaults().load_from_file(file.path()).unwrap();

        assert_eq!(config.crs.value, 3857);
        assert_eq!(config.crs.source, ConfigSource::File);
        assert_eq!(config.distance_unit.value, DistanceUnit::Kilometers);
        assert_eq!(config.geometry_validity.value, ValidityMode::Strict);
        assert_eq!(config.embedder.value, "ollama:custom-model");
    }

    #[test]
    fn test_cli_overrides() {
        let mut config = LayeredConfig::with_defaults();

        let overrides = CliConfigOverrides {
            crs: Some(32748),
            distance_unit: Some(DistanceUnit::Miles),
            geometry_validity: None,
            embedder: None,
        };

        config.update_from_cli(overrides);

        assert_eq!(config.crs.value, 32748);
        assert_eq!(config.crs.source, ConfigSource::Cli);
        assert_eq!(config.distance_unit.value, DistanceUnit::Miles);
        assert_eq!(config.distance_unit.source, ConfigSource::Cli);
        // These should still be defaults
        assert_eq!(config.geometry_validity.source, ConfigSource::Default);
        assert_eq!(config.embedder.source, ConfigSource::Default);
    }

    #[test]
    fn test_parse_distance_unit() {
        assert_eq!(parse_distance_unit("meters").unwrap(), DistanceUnit::Meters);
        assert_eq!(parse_distance_unit("m").unwrap(), DistanceUnit::Meters);
        assert_eq!(parse_distance_unit("KILOMETERS").unwrap(), DistanceUnit::Kilometers);
        assert_eq!(parse_distance_unit("miles").unwrap(), DistanceUnit::Miles);
        assert!(parse_distance_unit("invalid").is_err());
    }

    #[test]
    fn test_parse_validity_mode() {
        assert_eq!(parse_validity_mode("strict").unwrap(), ValidityMode::Strict);
        assert_eq!(parse_validity_mode("LENIENT").unwrap(), ValidityMode::Lenient);
        assert!(parse_validity_mode("invalid").is_err());
    }

    #[test]
    fn test_inspection_map() {
        let config = LayeredConfig::with_defaults();
        let map = config.to_inspection_map();

        assert!(map.contains_key("crs"));
        assert!(map.contains_key("distance_unit"));
        assert!(map.contains_key("geometry_validity"));
        assert!(map.contains_key("embedder"));

        let (crs_value, crs_source) = &map["crs"];
        assert_eq!(crs_value, "EPSG:4326");
        assert_eq!(*crs_source, ConfigSource::Default);
    }
}
