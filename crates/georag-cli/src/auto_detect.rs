#![allow(dead_code)]

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Auto-detect PostgreSQL installation and connection
pub struct PostgresDetection {
    pub installed: bool,
    pub running: bool,
    pub suggested_url: Option<String>,
    pub version: Option<String>,
}

impl PostgresDetection {
    pub fn detect() -> Self {
        let installed = Self::is_installed();
        let running = if installed { Self::is_running() } else { false };
        let version = if installed { Self::get_version() } else { None };
        let suggested_url = if running {
            Some(Self::suggest_connection_url())
        } else {
            None
        };

        Self {
            installed,
            running,
            suggested_url,
            version,
        }
    }

    fn is_installed() -> bool {
        Command::new("psql").arg("--version").output().is_ok()
    }

    fn is_running() -> bool {
        // Try to connect to default PostgreSQL
        Command::new("psql")
            .args(["-h", "localhost", "-U", "postgres", "-c", "SELECT 1"])
            .env("PGPASSWORD", "")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn get_version() -> Option<String> {
        Command::new("psql")
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
    }

    fn suggest_connection_url() -> String {
        // Check common locations
        let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());

        // Try localhost first
        format!("postgresql://{}@localhost:5432/georag", user)
    }

    pub fn check_extensions(&self, database_url: &str) -> ExtensionCheck {
        if !self.running {
            return ExtensionCheck {
                postgis: false,
                pgvector: false,
                can_check: false,
            };
        }

        // Try to check extensions
        let postgis = Self::check_extension(database_url, "postgis");
        let pgvector = Self::check_extension(database_url, "vector");

        ExtensionCheck { postgis, pgvector, can_check: true }
    }

    fn check_extension(database_url: &str, extension: &str) -> bool {
        // Parse URL to get connection params
        // This is simplified - in production, use proper URL parsing
        Command::new("psql")
            .arg(database_url)
            .arg("-c")
            .arg(format!("SELECT 1 FROM pg_extension WHERE extname = '{}'", extension))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Extension check result
pub struct ExtensionCheck {
    pub postgis: bool,
    pub pgvector: bool,
    pub can_check: bool,
}

/// Auto-detect dataset format and metadata
pub struct DatasetDetection {
    pub format: DatasetFormat,
    pub feature_count: Option<usize>,
    pub crs: Option<u32>,
    pub geometry_type: Option<String>,
    pub suggested_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatasetFormat {
    GeoJSON,
    Shapefile,
    GeoPackage,
    Unknown,
}

impl DatasetDetection {
    pub fn detect(path: &Path) -> Result<Self> {
        let format = Self::detect_format(path);
        let suggested_name = Self::suggest_name(path);

        let (feature_count, crs, geometry_type) = match format {
            DatasetFormat::GeoJSON => Self::analyze_geojson(path)?,
            _ => (None, None, None),
        };

        Ok(Self {
            format,
            feature_count,
            crs,
            geometry_type,
            suggested_name,
        })
    }

    fn detect_format(path: &Path) -> DatasetFormat {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "geojson" | "json" => DatasetFormat::GeoJSON,
                "shp" => DatasetFormat::Shapefile,
                "gpkg" => DatasetFormat::GeoPackage,
                _ => DatasetFormat::Unknown,
            }
        } else {
            DatasetFormat::Unknown
        }
    }

    fn suggest_name(path: &Path) -> String {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("dataset").to_string()
    }

    fn analyze_geojson(path: &Path) -> Result<(Option<usize>, Option<u32>, Option<String>)> {
        let content = std::fs::read_to_string(path)?;
        let geojson: serde_json::Value = serde_json::from_str(&content)?;

        let feature_count = geojson
            .get("features")
            .and_then(|f| f.as_array())
            .map(|features| features.len());

        let crs = geojson
            .get("crs")
            .and_then(|crs| crs.get("properties"))
            .and_then(|props| props.get("name"))
            .and_then(|name| name.as_str())
            .and_then(|name| name.split(':').next_back())
            .and_then(|code| code.parse::<u32>().ok())
            .or(Some(4326)); // Default to WGS84

        let geometry_type = geojson
            .get("features")
            .and_then(|f| f.as_array())
            .and_then(|arr| arr.first())
            .and_then(|feature| feature.get("geometry"))
            .and_then(|geom| geom.get("type"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        Ok((feature_count, crs, geometry_type))
    }
}

/// Auto-detect Ollama and available models
pub struct OllamaDetection {
    pub installed: bool,
    pub running: bool,
    pub available_models: Vec<String>,
}

impl OllamaDetection {
    pub fn detect() -> Self {
        let installed = Self::is_installed();
        let running = if installed { Self::is_running() } else { false };
        let available_models = if running {
            Self::list_models()
        } else {
            Vec::new()
        };

        Self { installed, running, available_models }
    }

    fn is_installed() -> bool {
        Command::new("ollama").arg("--version").output().is_ok()
    }

    fn is_running() -> bool {
        Command::new("ollama")
            .arg("list")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn list_models() -> Vec<String> {
        Command::new("ollama")
            .arg("list")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| {
                s.lines()
                    .skip(1) // Skip header
                    .filter_map(|line| line.split_whitespace().next())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn has_model(&self, model: &str) -> bool {
        self.available_models.iter().any(|m| m.contains(model))
    }
}

/// Detect workspace configuration issues
pub fn detect_workspace_issues(workspace_path: &Path) -> Vec<String> {
    let mut issues = Vec::new();

    let georag_dir = workspace_path.join(".georag");
    if !georag_dir.exists() {
        issues.push("Workspace not initialized (.georag directory missing)".to_string());
        return issues;
    }

    // Check config file
    let config_file = georag_dir.join("config.toml");
    if !config_file.exists() {
        issues.push("Configuration file missing (config.toml)".to_string());
    }

    // Check datasets directory
    let datasets_dir = georag_dir.join("datasets");
    if !datasets_dir.exists() {
        issues.push("Datasets directory missing".to_string());
    }

    // Check datasets.json
    let datasets_file = georag_dir.join("datasets.json");
    if !datasets_file.exists() {
        issues.push("Datasets registry missing (datasets.json)".to_string());
    } else {
        // Check if any datasets are registered
        if let Ok(content) = std::fs::read_to_string(&datasets_file) {
            if let Ok(datasets) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                if datasets.is_empty() {
                    issues.push("No datasets registered (run 'georag add')".to_string());
                }
            }
        }
    }

    // Check index
    let index_file = georag_dir.join("index").join("state.json");
    if !index_file.exists() {
        issues.push("Index not built (run 'georag build')".to_string());
    }

    issues
}
