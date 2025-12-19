//! Configuration loading utilities for CLI commands

use anyhow::{Context, Result};
use georag_core::config::{CliConfigOverrides, LayeredConfig};
use std::path::PathBuf;

/// Load layered configuration for a workspace
pub fn load_workspace_config(workspace_root: &PathBuf) -> Result<LayeredConfig> {
    let config_path = workspace_root.join(".georag").join("config.toml");
    
    let config = LayeredConfig::with_defaults()
        .load_from_file(&config_path)
        .context("Failed to load configuration file")?
        .load_from_env();
    
    Ok(config)
}

/// Load layered configuration with CLI overrides
pub fn load_workspace_config_with_overrides(
    workspace_root: &PathBuf,
    overrides: CliConfigOverrides,
) -> Result<LayeredConfig> {
    let mut config = load_workspace_config(workspace_root)?;
    config.update_from_cli(overrides);
    Ok(config)
}

/// Find the workspace root by looking for .georag directory
pub fn find_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        let georag_dir = current.join(".georag");
        if georag_dir.exists() && georag_dir.is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            anyhow::bail!("Not in a GeoRAG workspace. Run 'georag init' first.");
        }
    }
}
