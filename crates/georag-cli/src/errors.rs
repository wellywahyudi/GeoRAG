#![allow(dead_code)]

use console::style;
use std::fmt;

/// Enhanced error type with suggestions
pub struct CliError {
    pub message: String,
    pub context: Option<String>,
    pub suggestions: Vec<String>,
    pub help_command: Option<String>,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context: None,
            suggestions: Vec::new(),
            help_command: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    pub fn with_help(mut self, command: impl Into<String>) -> Self {
        self.help_command = Some(command.into());
        self
    }

    pub fn display(&self) {
        eprintln!("{} {}\n", style("✗").red().bold(), style(&self.message).red().bold());

        if let Some(ref context) = self.context {
            eprintln!("{}", context);
            eprintln!();
        }

        if !self.suggestions.is_empty() {
            eprintln!("{}", style("To fix this:").yellow().bold());
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, suggestion);
            }
            eprintln!();
        }

        if let Some(ref help_cmd) = self.help_command {
            eprintln!("{} {}", style("Need help?").cyan(), style(help_cmd).cyan().bold());
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

/// Create error for workspace not found
pub fn workspace_not_found() -> CliError {
    let current_dir = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    CliError::new("Not in a GeoRAG workspace")
        .with_context(format!(
            "You're not in a GeoRAG workspace directory.\n\nCurrent directory: {}\nLooking for: .georag directory",
            current_dir
        ))
        .with_suggestion("Initialize a workspace: georag init")
        .with_suggestion("Or navigate to an existing workspace")
        .with_help("Run: georag init --help")
}

/// Create error for database connection failure
pub fn database_connection_failed(error: &str) -> CliError {
    CliError::new("Cannot connect to PostgreSQL")
        .with_context(format!("DATABASE_URL is not set or connection failed.\n\nError: {}", error))
        .with_suggestion("Set DATABASE_URL: export DATABASE_URL=\"postgresql://localhost/georag\"")
        .with_suggestion("Or add to .georag/config.toml:\n  [postgres]\n  host = \"localhost\"\n  database = \"georag\"")
        .with_suggestion("Or use interactive setup: georag init --interactive")
        .with_help("Run: georag doctor")
}

/// Create error for missing dataset
pub fn dataset_not_found(path: &str) -> CliError {
    CliError::new("Dataset file not found")
        .with_context(format!("The specified dataset file does not exist.\n\nPath: {}", path))
        .with_suggestion("Check the file path and try again")
        .with_suggestion("Use absolute path or path relative to current directory")
        .with_help("Run: georag add --help")
}

/// Create error for CRS mismatch
pub fn crs_mismatch(dataset_crs: u32, workspace_crs: u32) -> CliError {
    CliError::new("CRS mismatch detected")
        .with_context(format!(
            "Dataset CRS (EPSG:{}) differs from workspace CRS (EPSG:{}).\n\nThis may cause spatial query issues.",
            dataset_crs, workspace_crs
        ))
        .with_suggestion("Reproject the dataset to match workspace CRS")
        .with_suggestion("Or use --force to add anyway (not recommended)")
        .with_suggestion("Or reinitialize workspace with matching CRS")
        .with_help("Run: georag add --help")
}

/// Create error for index not built
pub fn index_not_built() -> CliError {
    CliError::new("Index not built")
        .with_context("The retrieval index has not been built yet.\n\nYou need to build the index before querying.")
        .with_suggestion("Build the index: georag build")
        .with_suggestion("Check status: georag status")
        .with_help("Run: georag build --help")
}

/// Create error for no datasets
pub fn no_datasets() -> CliError {
    CliError::new("No datasets found")
        .with_context("No datasets have been added to the workspace.\n\nYou need at least one dataset to build an index.")
        .with_suggestion("Add a dataset: georag add dataset.geojson")
        .with_suggestion("Check current datasets: georag inspect datasets")
        .with_help("Run: georag add --help")
}

/// Create error for PostgreSQL not installed
pub fn postgres_not_installed() -> CliError {
    CliError::new("PostgreSQL not found")
        .with_context("PostgreSQL does not appear to be installed or running.")
        .with_suggestion("Install PostgreSQL: brew install postgresql (macOS)")
        .with_suggestion("Or: apt-get install postgresql (Ubuntu)")
        .with_suggestion("Start PostgreSQL: brew services start postgresql (macOS)")
        .with_suggestion("Or: sudo systemctl start postgresql (Linux)")
        .with_help("Run: georag doctor")
}

/// Create error for missing extensions
pub fn postgres_extensions_missing(missing: &[&str]) -> CliError {
    let extensions = missing.join(", ");
    CliError::new("PostgreSQL extensions missing")
        .with_context(format!(
            "Required PostgreSQL extensions are not installed.\n\nMissing: {}",
            extensions
        ))
        .with_suggestion("Install PostGIS: CREATE EXTENSION postgis;")
        .with_suggestion("Install pgvector: CREATE EXTENSION vector;")
        .with_suggestion("Run as PostgreSQL superuser (postgres)")
        .with_help("Run: georag doctor")
}

/// Create error for invalid configuration
pub fn invalid_config(key: &str, reason: &str) -> CliError {
    CliError::new(format!("Invalid configuration: {}", key))
        .with_context(format!("Configuration value is invalid.\n\nReason: {}", reason))
        .with_suggestion("Check .georag/config.toml for syntax errors")
        .with_suggestion("Or reinitialize: georag init --force")
        .with_help("Run: georag init --help")
}

/// Create error for embedder not available
pub fn embedder_not_available(embedder: &str) -> CliError {
    CliError::new("Embedder not available")
        .with_context(format!("The specified embedder is not available.\n\nEmbedder: {}", embedder))
        .with_suggestion("Check if Ollama is running: ollama list")
        .with_suggestion("Pull the model: ollama pull nomic-embed-text")
        .with_suggestion("Or use a different embedder: --embedder ollama:other-model")
        .with_help("Run: georag build --help")
}

/// Convert anyhow::Error to CliError with context
pub fn from_anyhow(error: anyhow::Error) -> CliError {
    let message = error.to_string();

    // Try to provide context based on error message
    if message.contains("No such file or directory") {
        CliError::new("File not found")
            .with_context(format!("Error: {}", message))
            .with_suggestion("Check the file path and try again")
    } else if message.contains("Connection refused") || message.contains("could not connect") {
        database_connection_failed(&message)
    } else if message.contains("permission denied") {
        CliError::new("Permission denied")
            .with_context(format!("Error: {}", message))
            .with_suggestion("Check file permissions")
            .with_suggestion("Or run with appropriate privileges")
    } else {
        CliError::new(message)
    }
}
