use crate::cli::InitArgs;
use crate::dry_run::{display_planned_actions, ActionType, PlannedAction};
use crate::interactive;
use crate::output::OutputWriter;
use crate::output_types::InitOutput;
use anyhow::{bail, Context, Result};
use georag_core::config::{parse_distance_unit, parse_validity_mode};
use georag_core::models::workspace::WorkspaceConfig;
use std::fs;

pub fn execute(args: InitArgs, output: &OutputWriter, dry_run: bool) -> Result<()> {
    // If interactive mode, use interactive prompts
    if args.interactive {
        let interactive_result = interactive::interactive_init()?;

        // Use the interactive results
        let distance_unit = parse_distance_unit(&interactive_result.distance_unit)?;
        let validity_mode = parse_validity_mode(&interactive_result.validity_mode)?;

        let config = WorkspaceConfig {
            crs: interactive_result.crs,
            distance_unit,
            geometry_validity: validity_mode,
        };

        // Create workspace with interactive settings
        create_workspace(&interactive_result.path, &config, output, dry_run)?;

        // If PostgreSQL was selected, create config file
        if interactive_result.use_postgres {
            if let Some(database_url) = interactive_result.database_url {
                crate::config::ConfigFile::create_default(
                    &interactive_result.path,
                    true,
                    Some(database_url),
                )?;
            }
        }

        return Ok(());
    }

    // Non-interactive mode (original behavior)
    let distance_unit = parse_distance_unit(&args.distance_unit)?;
    let validity_mode = parse_validity_mode(&args.validity_mode)?;

    let config = WorkspaceConfig {
        crs: args.crs,
        distance_unit,
        geometry_validity: validity_mode,
    };

    create_workspace(&args.path, &config, output, dry_run)
}

fn create_workspace(
    path: &std::path::Path,
    config: &WorkspaceConfig,
    output: &OutputWriter,
    dry_run: bool,
) -> Result<()> {
    // Check if workspace already exists
    let georag_dir = path.join(".georag");
    if georag_dir.exists() {
        bail!("Workspace already exists at {}. Use --force to overwrite", path.display());
    }

    if dry_run {
        let actions = vec![
            PlannedAction::new(
                ActionType::CreateDirectory,
                format!("Create .georag directory at {}", path.display()),
            ),
            PlannedAction::new(ActionType::CreateFile, "Create config.toml")
                .with_detail(format!("CRS: EPSG:{}", config.crs))
                .with_detail(format!("Distance Unit: {:?}", config.distance_unit))
                .with_detail(format!("Validity Mode: {:?}", config.geometry_validity)),
            PlannedAction::new(ActionType::CreateDirectory, "Create datasets directory"),
            PlannedAction::new(ActionType::CreateDirectory, "Create index directory"),
            PlannedAction::new(ActionType::CreateFile, "Create datasets.json (empty)"),
        ];

        display_planned_actions(output, &actions);
        return Ok(());
    }

    // Create .georag directory
    fs::create_dir_all(&georag_dir).context("Failed to create .georag directory")?;

    // Create config.toml
    let config_path = georag_dir.join("config.toml");
    let config_toml = format!(
        r#"# GeoRAG Workspace Configuration

# Coordinate Reference System (EPSG code)
# Common values:
#   4326 - WGS 84 (latitude/longitude)
#   3857 - Web Mercator
crs = {}

# Distance unit for spatial operations
# Options: "Meters", "Kilometers", "Miles", "Feet"
distance_unit = "{:?}"

# Geometry validity mode
# Options: "Strict" (reject invalid), "Lenient" (attempt to fix)
geometry_validity = "{:?}"
"#,
        config.crs, config.distance_unit, config.geometry_validity
    );

    fs::write(&config_path, config_toml).context("Failed to write config.toml")?;

    // Create datasets directory
    let datasets_dir = georag_dir.join("datasets");
    fs::create_dir_all(&datasets_dir).context("Failed to create datasets directory")?;

    // Create index directory
    let index_dir = georag_dir.join("index");
    fs::create_dir_all(&index_dir).context("Failed to create index directory")?;

    // Create datasets.json (empty array)
    let datasets_file = georag_dir.join("datasets.json");
    fs::write(&datasets_file, "[]").context("Failed to create datasets.json")?;

    // Output success message
    if output.is_json() {
        let json_output = InitOutput {
            workspace_path: path.display().to_string(),
            crs: config.crs,
            distance_unit: format!("{:?}", config.distance_unit),
            validity_mode: format!("{:?}", config.geometry_validity),
        };
        output.result(json_output)?;
    } else {
        output.success(format!("Initialized GeoRAG workspace at {}", path.display()));

        output.section("Configuration");
        output.kv("CRS", format!("EPSG:{}", config.crs));
        output.kv("Distance Unit", format!("{:?}", config.distance_unit));
        output.kv("Validity Mode", format!("{:?}", config.geometry_validity));
    }

    Ok(())
}
