use crate::batch::{display_file_progress, scan_directory, BatchSummary, FileProcessingResult};
use crate::cli::AddArgs;
use crate::dry_run::{display_planned_actions, ActionType, PlannedAction};
use crate::output::OutputWriter;
use crate::output_types::{AddOutput, CrsMismatchInfo};
use crate::storage::Storage;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use georag_core::formats::{
    docx::DocxReader, geojson::GeoJsonReader, gpx::GpxReader, kml::KmlReader, pdf::PdfReader,
    shapefile::ShapefileFormatReader, FormatRegistry,
};
use georag_core::models::dataset::GeometryType;
use georag_core::models::{Dataset, DatasetId};
use std::fs;
use std::path::PathBuf;

pub async fn execute(
    args: AddArgs,
    output: &OutputWriter,
    dry_run: bool,
    storage: &Storage,
) -> Result<()> {
    if !args.path.exists() {
        bail!("Path not found: {}", args.path.display());
    }

    // Register all format readers
    let mut registry = FormatRegistry::new();
    registry.register(Box::new(GeoJsonReader));
    registry.register(Box::new(ShapefileFormatReader));
    registry.register(Box::new(GpxReader));
    registry.register(Box::new(KmlReader));
    registry.register(Box::new(PdfReader));
    registry.register(Box::new(DocxReader));

    if args.path.is_dir() {
        // Batch processing mode
        execute_batch(args, output, dry_run, storage, &registry).await
    } else {
        // Single file mode
        execute_single(args, output, dry_run, storage, &registry).await
    }
}

/// Execute batch processing for a directory
async fn execute_batch(
    args: AddArgs,
    output: &OutputWriter,
    dry_run: bool,
    storage: &Storage,
    registry: &FormatRegistry,
) -> Result<()> {
    output.info(format!("Scanning directory: {}", args.path.display()));

    let discovered_files =
        scan_directory(&args.path, registry, false).context("Failed to scan directory")?;

    if discovered_files.is_empty() {
        output.warning("No supported files found in directory");
        return Ok(());
    }

    output.info(format!("Found {} supported files", discovered_files.len()));

    if dry_run {
        let mut actions = vec![PlannedAction::new(
            ActionType::ModifyFile,
            format!("Process {} files from directory", discovered_files.len()),
        )];

        for file in &discovered_files {
            actions.push(PlannedAction::new(
                ActionType::CopyFile,
                format!("Add {} ({})", file.path.display(), file.format_name),
            ));
        }

        display_planned_actions(output, &actions);
        return Ok(());
    }

    let mut summary = BatchSummary::new();
    summary.total_files = discovered_files.len();

    // Process files sequentially or in parallel based on args
    if args.parallel {
        for (idx, file) in discovered_files.iter().enumerate() {
            display_file_progress(output, idx + 1, discovered_files.len(), file);

            // Create args for single file processing
            let file_args = AddArgs {
                path: file.path.clone(),
                name: None, // Use default name from filename
                force: args.force,
                interactive: false, // Disable interactive mode in batch
                track_type: args.track_type.clone(),
                folder: args.folder.clone(),
                geometry: args.geometry.clone(),
                parallel: false,
            };

            // Process the file
            match execute_single(file_args, output, false, storage, registry).await {
                Ok(_) => {
                    summary.add_success(FileProcessingResult {
                        path: file.path.clone(),
                        format_name: file.format_name.clone(),
                        success: true,
                        error: None,
                        dataset_name: Some(
                            file.path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                        ),
                    });
                }
                Err(e) => {
                    summary.add_failure(FileProcessingResult {
                        path: file.path.clone(),
                        format_name: file.format_name.clone(),
                        success: false,
                        error: Some(e.to_string()),
                        dataset_name: None,
                    });
                }
            }
        }
    } else {
        for (idx, file) in discovered_files.iter().enumerate() {
            display_file_progress(output, idx + 1, discovered_files.len(), file);

            let file_args = AddArgs {
                path: file.path.clone(),
                name: None,
                force: args.force,
                interactive: false,
                track_type: args.track_type.clone(),
                folder: args.folder.clone(),
                geometry: args.geometry.clone(),
                parallel: false,
            };

            match execute_single(file_args, output, false, storage, registry).await {
                Ok(_) => {
                    summary.add_success(FileProcessingResult {
                        path: file.path.clone(),
                        format_name: file.format_name.clone(),
                        success: true,
                        error: None,
                        dataset_name: Some(
                            file.path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                        ),
                    });
                }
                Err(e) => {
                    summary.add_failure(FileProcessingResult {
                        path: file.path.clone(),
                        format_name: file.format_name.clone(),
                        success: false,
                        error: Some(e.to_string()),
                        dataset_name: None,
                    });
                }
            }
        }
    }

    // Display summary
    summary.display(output);

    // Return error if any files failed (but still show summary)
    if !summary.all_succeeded() {
        bail!("{} of {} files failed to process", summary.failure_count(), summary.total_files);
    }

    Ok(())
}

/// Execute single file processing
async fn execute_single(
    args: AddArgs,
    output: &OutputWriter,
    dry_run: bool,
    storage: &Storage,
    registry: &FormatRegistry,
) -> Result<()> {
    // Check if dataset file exists
    if !args.path.exists() {
        bail!("Dataset file not found: {}", args.path.display());
    }

    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    // Load workspace config
    let config_path = georag_dir.join("config.toml");
    let config_content = fs::read_to_string(&config_path).context("Failed to read config.toml")?;
    let config: georag_core::models::WorkspaceConfig =
        toml::from_str(&config_content).context("Failed to parse config.toml")?;

    // Detect format
    let reader = registry.detect_format(&args.path).context("Failed to detect file format")?;

    output.info(format!("Detected format: {}", reader.format_name()));

    // Validate format
    let validation = reader.validate(&args.path).await.context("Failed to validate file")?;

    if !validation.is_valid() {
        for error in &validation.errors {
            output.error(error.clone());
        }
        bail!("Format validation failed");
    }

    for warning in &validation.warnings {
        output.warning(warning.clone());
    }

    // Build format options from CLI arguments
    let mut format_options = georag_core::formats::FormatOptions::new();

    if let Some(track_type) = &args.track_type {
        format_options = format_options.with_option("track_type", track_type);
        output.info(format!("GPX track type filter: {}", track_type));
    }

    if let Some(folder) = &args.folder {
        format_options = format_options.with_option("folder", folder);
        output.info(format!("KML folder filter: {}", folder));
    }

    // Read dataset using format reader with options and optional geometry association
    let format_dataset = if let Some(geometry_arg) = &args.geometry {
        // Parse geometry argument
        let geometry =
            parse_geometry_argument(geometry_arg).context("Failed to parse geometry argument")?;

        output.info("Associating geometry with document".to_string());

        // Read with geometry association
        reader
            .read_with_geometry(&args.path, geometry)
            .await
            .context("Failed to read dataset with geometry")?
    } else if format_options.options.is_empty() {
        reader.read(&args.path).await.context("Failed to read dataset")?
    } else {
        reader
            .read_with_options(&args.path, &format_options)
            .await
            .context("Failed to read dataset")?
    };

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
        args.path.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed").to_string()
    });

    if dry_run {
        let mut actions = vec![
            PlannedAction::new(ActionType::ModifyFile, "Store dataset in database")
                .with_detail(format!("Add dataset: {}", dataset_name))
                .with_detail(format!("Format: {}", format_dataset.format_metadata.format_name))
                .with_detail(format!("Geometry Type: {:?}", geometry_type))
                .with_detail(format!("Feature Count: {}", feature_count))
                .with_detail(format!("CRS: EPSG:{}", crs)),
            PlannedAction::new(ActionType::CopyFile, "Copy dataset file to workspace".to_string())
                .with_detail(format!("Source: {}", args.path.display()))
                .with_detail("Destination: .georag/datasets/".to_string()),
        ];

        // Add format-specific metadata to dry-run output
        if let Some(layer_name) = &format_dataset.format_metadata.layer_name {
            actions[0] = actions[0].clone().with_detail(format!("Layer: {}", layer_name));
        }
        if let Some(page_count) = format_dataset.format_metadata.page_count {
            actions[0] = actions[0].clone().with_detail(format!("Pages: {}", page_count));
        }
        if let Some(paragraph_count) = format_dataset.format_metadata.paragraph_count {
            actions[0] = actions[0].clone().with_detail(format!("Paragraphs: {}", paragraph_count));
        }

        if crs != config.crs {
            actions.insert(
                0,
                PlannedAction::new(ActionType::ModifyFile, "CRS mismatch warning")
                    .with_detail(format!("Dataset CRS: EPSG:{}", crs))
                    .with_detail(format!("Workspace CRS: EPSG:{}", config.crs)),
            );
        }

        display_planned_actions(output, &actions);
        return Ok(());
    }

    // Create dataset object
    let dataset = Dataset {
        id: DatasetId(0),
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
            spatial_association: None,
        },
        added_at: Utc::now(),
    };

    // Store dataset using SpatialStore trait
    let dataset_id = storage.spatial.store_dataset(&dataset).await?;

    // Copy dataset file to workspace (for backward compatibility with file-based operations)
    let datasets_dir = georag_dir.join("datasets");
    fs::create_dir_all(&datasets_dir)?;
    let dataset_filename =
        format!("{}_{}", dataset_id.0, args.path.file_name().unwrap().to_str().unwrap());
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
        output.kv("Format", &format_dataset.format_metadata.format_name);
        output.kv("Geometry Type", format!("{:?}", geometry_type));
        output.kv("Feature Count", feature_count);
        output.kv("CRS", format!("EPSG:{}", crs));

        // Show format-specific metadata
        if let Some(layer_name) = &format_dataset.format_metadata.layer_name {
            output.kv("Layer", layer_name);
        }
        if let Some(page_count) = format_dataset.format_metadata.page_count {
            output.kv("Pages", page_count);
        }
        if let Some(paragraph_count) = format_dataset.format_metadata.paragraph_count {
            output.kv("Paragraphs", paragraph_count);
        }
        if let Some(extraction_method) = &format_dataset.format_metadata.extraction_method {
            output.kv("Extraction Method", extraction_method);
        }
        if let Some(spatial_assoc) = &format_dataset.format_metadata.spatial_association {
            output.kv("Spatial Association", &spatial_assoc.source);
            if let Some(desc) = &spatial_assoc.description {
                output.kv("Association Details", desc);
            }
        }

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
    let content = fs::read_to_string(path).context("Failed to read dataset file")?;

    // Parse as GeoJSON
    let geojson: geojson::GeoJson = content.parse().context("Failed to parse GeoJSON")?;

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
            let crs = fc
                .foreign_members
                .as_ref()
                .and_then(|fm| fm.get("crs"))
                .and_then(extract_epsg_from_crs)
                .unwrap_or(4326);

            Ok((geometry_type, feature_count, crs))
        }
        geojson::GeoJson::Feature(feature) => {
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
                if let Some(epsg_str) = name_str.split(':').next_back() {
                    return epsg_str.parse().ok();
                }
            }
        }
    }
    None
}

/// Parse geometry argument - can be inline GeoJSON or path to file
fn parse_geometry_argument(geometry_arg: &str) -> Result<serde_json::Value> {
    // Try to parse as JSON first (inline geometry)
    if let Ok(geom) = serde_json::from_str::<serde_json::Value>(geometry_arg) {
        // Validate it's a valid GeoJSON geometry
        if geom.get("type").is_some() && geom.get("coordinates").is_some() {
            return Ok(geom);
        }
    }

    // Try to read as file path
    let path = PathBuf::from(geometry_arg);
    if path.exists() {
        let content = fs::read_to_string(&path).context("Failed to read geometry file")?;

        // Parse the file content
        let geojson: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse geometry file as JSON")?;

        // Extract geometry from GeoJSON
        if let Some(geom_type) = geojson.get("type") {
            match geom_type.as_str() {
                Some("Feature") => {
                    // Extract geometry from Feature
                    if let Some(geometry) = geojson.get("geometry") {
                        return Ok(geometry.clone());
                    }
                }
                Some("FeatureCollection") => {
                    // Use geometry from first feature
                    if let Some(features) = geojson.get("features").and_then(|f| f.as_array()) {
                        if let Some(first_feature) = features.first() {
                            if let Some(geometry) = first_feature.get("geometry") {
                                return Ok(geometry.clone());
                            }
                        }
                    }
                }
                Some("Point")
                | Some("LineString")
                | Some("Polygon")
                | Some("MultiPoint")
                | Some("MultiLineString")
                | Some("MultiPolygon") => {
                    // It's already a geometry
                    return Ok(geojson);
                }
                _ => {}
            }
        }

        bail!("Geometry file does not contain valid GeoJSON geometry");
    }

    bail!("Geometry argument must be valid GeoJSON geometry string or path to GeoJSON file");
}
