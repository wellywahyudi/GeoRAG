//! Inspect command implementation

use crate::cli::{InspectArgs, InspectTarget};
use crate::output::OutputWriter;
use crate::output_types::{
    InspectDatasetsOutput, DatasetInfo, InspectIndexOutput, InspectCrsOutput,
    DatasetCrsInfo, InspectConfigOutput, ConfigValue,
};
use anyhow::{bail, Context, Result};
use georag_core::models::{DatasetMeta, WorkspaceConfig};
use georag_core::models::workspace::IndexState;
use std::fs;
use std::path::PathBuf;
use tabled::Tabled;

pub fn execute(args: InspectArgs, output: &OutputWriter) -> Result<()> {
    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    match args.target {
        InspectTarget::Datasets => inspect_datasets(&georag_dir, output)?,
        InspectTarget::Index => inspect_index(&georag_dir, output)?,
        InspectTarget::Crs => inspect_crs(&georag_dir, output)?,
        InspectTarget::Config => inspect_config(&georag_dir, output)?,
    }
    
    Ok(())
}

/// Inspect registered datasets
fn inspect_datasets(georag_dir: &PathBuf, output: &OutputWriter) -> Result<()> {
    let datasets = load_datasets(georag_dir)?;

    if datasets.is_empty() {
        if output.is_json() {
            output.result(InspectDatasetsOutput { datasets: vec![] })?;
        } else {
            output.info("No datasets registered");
        }
        return Ok(());
    }

    if output.is_json() {
        let dataset_infos: Vec<DatasetInfo> = datasets
            .iter()
            .map(|d| DatasetInfo {
                id: d.id.0,
                name: d.name.clone(),
                geometry_type: d.geometry_type,
                feature_count: d.feature_count,
                crs: d.crs,
                added_at: d.added_at,
            })
            .collect();
        
        output.result(InspectDatasetsOutput { datasets: dataset_infos })?;
    } else {
        output.section("Registered Datasets");
        
        #[derive(Tabled)]
        struct DatasetRow {
            #[tabled(rename = "ID")]
            id: u64,
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Type")]
            geometry_type: String,
            #[tabled(rename = "Features")]
            feature_count: usize,
            #[tabled(rename = "CRS")]
            crs: String,
        }

        let rows: Vec<DatasetRow> = datasets
            .iter()
            .map(|d| DatasetRow {
                id: d.id.0,
                name: d.name.clone(),
                geometry_type: format!("{:?}", d.geometry_type),
                feature_count: d.feature_count,
                crs: format!("EPSG:{}", d.crs),
            })
            .collect();

        output.table(rows);
    }

    Ok(())
}

/// Inspect index metadata
fn inspect_index(georag_dir: &PathBuf, output: &OutputWriter) -> Result<()> {
    let state_path = georag_dir.join("index").join("state.json");
    
    if !state_path.exists() {
        if output.is_json() {
            output.result(InspectIndexOutput {
                built: false,
                hash: None,
                built_at: None,
                embedder: None,
                chunk_count: None,
                embedding_dim: None,
            })?;
        } else {
            output.info("Index not built. Run 'georag build' first.");
        }
        return Ok(());
    }

    let state = load_index_state(georag_dir)?;

    if output.is_json() {
        output.result(InspectIndexOutput {
            built: true,
            hash: Some(state.hash),
            built_at: Some(state.built_at),
            embedder: Some(state.embedder),
            chunk_count: Some(state.chunk_count),
            embedding_dim: Some(state.embedding_dim),
        })?;
    } else {
        output.section("Index Metadata");
        output.kv("Hash", &state.hash);
        output.kv("Built At", state.built_at.format("%Y-%m-%d %H:%M:%S UTC"));
        output.kv("Embedder", &state.embedder);
        output.kv("Chunk Count", state.chunk_count);
        output.kv("Embedding Dimension", state.embedding_dim);
    }

    Ok(())
}

/// Inspect CRS information
fn inspect_crs(georag_dir: &PathBuf, output: &OutputWriter) -> Result<()> {
    let config = load_workspace_config(georag_dir)?;
    let datasets = load_datasets(georag_dir)?;

    if output.is_json() {
        let dataset_crs_infos: Vec<DatasetCrsInfo> = datasets
            .iter()
            .map(|d| DatasetCrsInfo {
                name: d.name.clone(),
                crs: d.crs,
                matches_workspace: d.crs == config.crs,
            })
            .collect();
        
        output.result(InspectCrsOutput {
            workspace_crs: config.crs,
            datasets: dataset_crs_infos,
        })?;
    } else {
        output.section("Workspace CRS");
        output.kv("EPSG Code", config.crs);
        output.kv("Distance Unit", format!("{:?}", config.distance_unit));

        if !datasets.is_empty() {
            output.section("Dataset CRS");
            
            #[derive(Tabled)]
            struct CrsRow {
                #[tabled(rename = "Dataset")]
                name: String,
                #[tabled(rename = "CRS")]
                crs: String,
                #[tabled(rename = "Match")]
                matches: String,
            }

            let rows: Vec<CrsRow> = datasets
                .iter()
                .map(|d| CrsRow {
                    name: d.name.clone(),
                    crs: format!("EPSG:{}", d.crs),
                    matches: if d.crs == config.crs { "✓" } else { "✗" }.to_string(),
                })
                .collect();

            output.table(rows);
        }
    }

    Ok(())
}

/// Inspect configuration
fn inspect_config(georag_dir: &PathBuf, output: &OutputWriter) -> Result<()> {
    use georag_core::config::LayeredConfig;
    
    let config_path = georag_dir.join("config.toml");
    
    // Build layered configuration
    let layered_config = LayeredConfig::with_defaults()
        .load_from_file(&config_path)
        .unwrap_or_else(|_| LayeredConfig::with_defaults())
        .load_from_env();

    let inspection_map = layered_config.to_inspection_map();

    if output.is_json() {
        // Extract specific config values for structured output
        let crs_entry = inspection_map.get("crs")
            .map(|(v, s)| ConfigValue {
                value: v.parse::<u32>().unwrap_or(4326),
                source: format!("{:?}", s),
            })
            .unwrap_or(ConfigValue {
                value: 4326,
                source: "Default".to_string(),
            });
        
        let distance_unit_entry = inspection_map.get("distance_unit")
            .map(|(v, s)| ConfigValue {
                value: v.clone(),
                source: format!("{:?}", s),
            })
            .unwrap_or(ConfigValue {
                value: "Meters".to_string(),
                source: "Default".to_string(),
            });
        
        let geometry_validity_entry = inspection_map.get("geometry_validity")
            .map(|(v, s)| ConfigValue {
                value: v.clone(),
                source: format!("{:?}", s),
            })
            .unwrap_or(ConfigValue {
                value: "Lenient".to_string(),
                source: "Default".to_string(),
            });
        
        let embedder_entry = inspection_map.get("embedder")
            .map(|(v, s)| ConfigValue {
                value: v.clone(),
                source: format!("{:?}", s),
            })
            .unwrap_or(ConfigValue {
                value: "ollama:nomic-embed-text".to_string(),
                source: "Default".to_string(),
            });
        
        output.result(InspectConfigOutput {
            crs: crs_entry,
            distance_unit: distance_unit_entry,
            geometry_validity: geometry_validity_entry,
            embedder: embedder_entry,
        })?;
    } else {
        output.section("Configuration Values");
        
        #[derive(Tabled)]
        struct ConfigRow {
            #[tabled(rename = "Key")]
            key: String,
            #[tabled(rename = "Value")]
            value: String,
            #[tabled(rename = "Source")]
            source: String,
        }

        let mut rows: Vec<ConfigRow> = inspection_map
            .into_iter()
            .map(|(key, (value, source))| ConfigRow {
                key,
                value,
                source: format!("{:?}", source),
            })
            .collect();
        
        // Sort by key for consistent output
        rows.sort_by(|a, b| a.key.cmp(&b.key));

        output.table(rows);

        output.section("Configuration Precedence");
        output.info("CLI arguments > Environment variables > Config file > Defaults");
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

/// Load index state
fn load_index_state(georag_dir: &PathBuf) -> Result<IndexState> {
    let state_path = georag_dir.join("index").join("state.json");
    let content = fs::read_to_string(&state_path)?;
    let state: IndexState = serde_json::from_str(&content)?;
    Ok(state)
}
