//! Status command implementation

use crate::cli::StatusArgs;
use crate::output::OutputWriter;
use crate::output_types::{StatusOutput, IndexStatus, StorageStatus};
use anyhow::{bail, Context, Result};
use georag_core::models::{DatasetMeta, WorkspaceConfig};
use georag_core::models::workspace::IndexState;
use std::fs;
use std::path::PathBuf;

pub fn execute(args: StatusArgs, output: &OutputWriter) -> Result<()> {
    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    // Load workspace config
    let config = load_workspace_config(&georag_dir)?;

    // Load datasets
    let datasets = load_datasets(&georag_dir)?;

    // Check index status
    let index_state = load_index_state_optional(&georag_dir);

    // Prepare storage status if verbose
    let storage_status = if args.verbose {
        let datasets_dir = georag_dir.join("datasets");
        let index_dir = georag_dir.join("index");
        Some(StorageStatus {
            datasets_dir: datasets_dir.exists(),
            index_dir: index_dir.exists(),
        })
    } else {
        None
    };

    // Display status
    if output.is_json() {
        let index_status = if let Some(state) = index_state {
            IndexStatus {
                built: true,
                hash: Some(state.hash),
                built_at: Some(state.built_at),
                embedder: Some(state.embedder),
                chunk_count: Some(state.chunk_count),
                embedding_dim: if args.verbose { Some(state.embedding_dim) } else { None },
            }
        } else {
            IndexStatus {
                built: false,
                hash: None,
                built_at: None,
                embedder: None,
                chunk_count: None,
                embedding_dim: None,
            }
        };

        let json_output = StatusOutput {
            workspace_path: workspace_root.display().to_string(),
            crs: config.crs,
            distance_unit: format!("{:?}", config.distance_unit),
            dataset_count: datasets.len(),
            index: index_status,
            storage: storage_status,
        };
        output.result(json_output)?;
    } else {
        output.section("Workspace Status");
        output.kv("Location", workspace_root.display());
        output.kv("CRS", format!("EPSG:{}", config.crs));
        output.kv("Distance Unit", format!("{:?}", config.distance_unit));
        output.kv("Datasets", datasets.len());

        output.section("Index Status");
        if let Some(state) = index_state {
            output.kv("Status", "Built");
            output.kv("Hash", &state.hash);
            output.kv("Built At", state.built_at.format("%Y-%m-%d %H:%M:%S UTC"));
            output.kv("Embedder", &state.embedder);
            output.kv("Chunks", state.chunk_count);
            
            if args.verbose {
                output.kv("Embedding Dimension", state.embedding_dim);
            }
        } else {
            output.kv("Status", "Not built");
            output.info("Run 'georag build' to create the index");
        }

        // Check adapter status (simplified - just check if directories exist)
        if args.verbose {
            output.section("Storage Status");
            if let Some(storage) = storage_status {
                output.kv("Datasets Directory", if storage.datasets_dir { "✓" } else { "✗" });
                output.kv("Index Directory", if storage.index_dir { "✓" } else { "✗" });
            }
        }
    }

    Ok(())
}

/// Find the workspace root by looking for .georag directory
fn find_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let georag_dir = current.join(".georag");
        if georag_dir.exists() && georag_dir.is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("Not in a GeoRAG workspace. Run 'georag init' first.");
        }
    }
}

/// Load workspace configuration
fn load_workspace_config(georag_dir: &PathBuf) -> Result<WorkspaceConfig> {
    let config_path = georag_dir.join("config.toml");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read config.toml")?;
    let config: WorkspaceConfig = toml::from_str(&config_content)
        .context("Failed to parse config.toml")?;
    Ok(config)
}

/// Load datasets from datasets.json
fn load_datasets(georag_dir: &PathBuf) -> Result<Vec<DatasetMeta>> {
    let datasets_file = georag_dir.join("datasets.json");
    if !datasets_file.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&datasets_file)?;
    let datasets: Vec<DatasetMeta> = serde_json::from_str(&content)?;
    Ok(datasets)
}

/// Load index state (returns None if not built)
fn load_index_state_optional(georag_dir: &PathBuf) -> Option<IndexState> {
    let state_path = georag_dir.join("index").join("state.json");
    if !state_path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(&state_path).ok()?;
    let state: IndexState = serde_json::from_str(&content).ok()?;
    Some(state)
}
