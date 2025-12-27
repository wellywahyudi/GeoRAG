//! Add command implementation

use crate::cli::AddArgs;
use crate::dry_run::{display_planned_actions, ActionType, PlannedAction};
use crate::output::OutputWriter;
use crate::output_types::{AddOutput, CrsMismatchInfo};
use crate::storage::Storage;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use georag_core::formats::{FormatRegistry, geojson::GeoJsonReader};
use georag_core::models::{Dataset, DatasetId};
use georag_core::models::dataset::GeometryType;
use std::fs;
use std::path::PathBuf;

pub async fn execute(args: AddArgs, output: &OutputWriter, dry_run: bool, storage: &Storage) -> Result<()> {
    // Check if dataset file exists
    if !args.path.exists() {
        bail!("Dataset file not found: {}", args.path.display());
    }

    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    // Load workspace config
    let config_path = georag_dir.join("config.toml");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read config.toml")?;
    let config: georag_core::models::WorkspaceConfig = toml::from_str(&config_content)
        .context("Failed to parse config.toml")?;

    // Create format registry and register GeoJSON reader
    let mut registry = FormatRegistry::new();
    registry.register(Box::new(GeoJsonReader));

    // Detect format
    let reader = registry.detect_format(&args.path)
        .context("Failed to detect file format")?;

    output.info(format!("Detected format: {}", reader.format_name()));

    // Validate format
    let validation = reader.validate(&args.path).await
        .context("Failed to validate file")?;

    if !validation.is_valid() {
        for error in &validation.errors {
            output.error(error.clone());
        }
        bail!("Format validation failed");
    }

    for warning in &validation.warnings {
        output.warning(warning.clone());
    }

    // Read dataset using format reader
    let format_dataset = reader.read(&args.path).await
        .context("Failed to read dataset")?;

    // Read and validate the dataset metadata (for backward compatibility)
    let (geometry_type, feature_count, crs) = read_dataset_metadata(&args.path)?;

    // Check for CRS mismatch
    if crs != config.crs && !args.force {
        output.warning(format!(
            "CRS mismatch: dataset has EPSG:{}, workspace expects EPSG:{}",
            crs, config.crs
        ));
        bail!("Use --force to add dataset with mismatched CRS, or reproject the dataset first");
    }

    // Determine dataset name
    let dataset_name = args.name.unwrap_or_else(|| {
        args.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    if dry_run {
        let mut actions = vec![
            PlannedAction::new(
                ActionType::ModifyFile,
                "Store dataset in database"
            )
            .with_detail(format!("Add dataset: {}", dataset_name))
            .with_detail(format!("Format: {}", format_dataset.format_metadata.format_name))
            .with_detail(format!("Geometry Type: {:?}", geometry_type))
            .with_detail(format!("Feature Count: {}", feature_count))
            .with_detail(format!("CRS: EPSG:{}", crs)),
            PlannedAction::new(
                ActionType::CopyFile,
                format!("Copy dataset file to workspace")
            )
            .with_detail(format!("Source: {}", args.path.display()))
            .with_detail(format!("Destination: .georag/datasets/")),
        ];
        
        if crs != config.crs {
            actions.insert(0, PlannedAction::new(
                ActionType::ModifyFile,
                "CRS mismatch warning"
            )
            .with_detail(format!("Dataset CRS: EPSG:{}", crs))
            .with_detail(format!("Workspace CRS: EPSG:{}", config.crs)));
        }
        
        display_planned_actions(output, &actions);
        return Ok(());
    }

    // Create dataset object
    let dataset = Dataset {
        id: DatasetId(0), // Will be assigned by storage
        name: dataset_name.clone(),
        path: args.path.clone(),
        geometry_type,
        feature_count,
        crs,
        format: georag_core::models::dataset::FormatMetadata {
            format_name: format_dataset.format_metadata.format_name.clone(),
            format_version: format_dataset.format_metadata.format_version.clone(),
            layer_name: format_dataset.format_metadata.layer_name.clone(),
            page_count: format_dataset.format_metadata.page_count,
            paragraph_count: format_dataset.format_metadata.paragraph_count,
            extraction_method: format_dataset.format_metadata.extraction_method.clone(),
            spatial_association: None, // No spatial association by default
        },
        added_at: Utc::now(),
    };

    // Store dataset using SpatialStore trait
    let dataset_id = storage.spatial.store_dataset(&dataset).await?;

    // Copy dataset file to workspace (for backward compatibility with file-based operations)
    let datasets_dir = georag_dir.join("datasets");
    fs::create_dir_all(&datasets_dir)?;
    let dataset_filename = format!("{}_{}", dataset_id.0, args.path.file_name().unwrap().to_str().unwrap());
    let dest_path = datasets_dir.join(&dataset_filename);
    fs::copy(&args.path, &dest_path)?;

    // Output success
    if output.is_json() {
        let crs_mismatch = if crs != config.crs {
            Some(CrsMismatchInfo {
                dataset_crs: crs,
                workspace_crs: config.crs,
            })
        } else {
            None
        };
        
        let json_output = AddOutput {
            dataset_name: dataset_name.clone(),
            geometry_type,
            feature_count,
            crs,
            crs_mismatch,
        };
        output.result(json_output)?;
    } else {
        output.success(format!("Added dataset: {}", dataset_name));
        output.section("Dataset Information");
        output.kv("Format", format_dataset.format_metadata.format_name);
        output.kv("Geometry Type", format!("{:?}", geometry_type));
        output.kv("Feature Count", feature_count);
        output.kv("CRS", format!("EPSG:{}", crs));

        if crs != config.crs {
            output.warning(format!(
                "Dataset CRS (EPSG:{}) differs from workspace CRS (EPSG:{})",
                crs, config.crs
            ));
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

/// Read dataset metadata from a GeoJSON file
fn read_dataset_metadata(path: &PathBuf) -> Result<(GeometryType, usize, u32)> {
    // Read the file
    let content = fs::read_to_string(path)
        .context("Failed to read dataset file")?;

    // Parse as GeoJSON
    let geojson: geojson::GeoJson = content.parse()
        .context("Failed to parse GeoJSON")?;

    // Extract metadata
    match geojson {
        geojson::GeoJson::FeatureCollection(fc) => {
            let feature_count = fc.features.len();
            
            // Determine geometry type from first feature
            let geometry_type = if let Some(first_feature) = fc.features.first() {
                if let Some(ref geom) = first_feature.geometry {
                    match &geom.value {
                        geojson::Value::Point(_) => GeometryType::Point,
                        geojson::Value::LineString(_) => GeometryType::LineString,
                        geojson::Value::Polygon(_) => GeometryType::Polygon,
                        geojson::Value::MultiPoint(_) => GeometryType::MultiPoint,
                        geojson::Value::MultiLineString(_) => GeometryType::MultiLineString,
                        geojson::Value::MultiPolygon(_) => GeometryType::MultiPolygon,
                        geojson::Value::GeometryCollection(_) => GeometryType::GeometryCollection,
                    }
                } else {
                    GeometryType::Point // Default
                }
            } else {
                GeometryType::Point // Default for empty collection
            };

            // Extract CRS (default to WGS84 if not specified)
            let crs = fc.foreign_members
                .as_ref()
                .and_then(|fm| fm.get("crs"))
                .and_then(|crs_obj| extract_epsg_from_crs(crs_obj))
                .unwrap_or(4326);

            Ok((geometry_type, feature_count, crs))
        }
        geojson::GeoJson::Feature(feature) => {
            // Single feature
            let geometry_type = if let Some(ref geom) = feature.geometry {
                match &geom.value {
                    geojson::Value::Point(_) => GeometryType::Point,
                    geojson::Value::LineString(_) => GeometryType::LineString,
                    geojson::Value::Polygon(_) => GeometryType::Polygon,
                    geojson::Value::MultiPoint(_) => GeometryType::MultiPoint,
                    geojson::Value::MultiLineString(_) => GeometryType::MultiLineString,
                    geojson::Value::MultiPolygon(_) => GeometryType::MultiPolygon,
                    geojson::Value::GeometryCollection(_) => GeometryType::GeometryCollection,
                }
            } else {
                GeometryType::Point
            };

            Ok((geometry_type, 1, 4326))
        }
        geojson::GeoJson::Geometry(geom) => {
            // Single geometry
            let geometry_type = match &geom.value {
                geojson::Value::Point(_) => GeometryType::Point,
                geojson::Value::LineString(_) => GeometryType::LineString,
                geojson::Value::Polygon(_) => GeometryType::Polygon,
                geojson::Value::MultiPoint(_) => GeometryType::MultiPoint,
                geojson::Value::MultiLineString(_) => GeometryType::MultiLineString,
                geojson::Value::MultiPolygon(_) => GeometryType::MultiPolygon,
                geojson::Value::GeometryCollection(_) => GeometryType::GeometryCollection,
            };

            Ok((geometry_type, 1, 4326))
        }
    }
}

/// Extract EPSG code from CRS object
fn extract_epsg_from_crs(crs: &serde_json::Value) -> Option<u32> {
    // Try to extract from properties.name
    if let Some(props) = crs.get("properties") {
        if let Some(name) = props.get("name") {
            if let Some(name_str) = name.as_str() {
                // Parse "EPSG:4326" or "urn:ogc:def:crs:EPSG::4326"
                if let Some(epsg_str) = name_str.split(':').last() {
                    return epsg_str.parse().ok();
                }
            }
        }
    }
    None
}
