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
