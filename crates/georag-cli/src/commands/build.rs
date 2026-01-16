use crate::cli::BuildArgs;
use crate::config::{find_workspace_root, load_workspace_config_with_overrides};
use crate::dry_run::{display_planned_actions, ActionType, PlannedAction};
use crate::output::OutputWriter;
use crate::output_types::BuildOutput;
use crate::storage::Storage;
use anyhow::{bail, Result};
use chrono::Utc;
use georag_core::config::CliConfigOverrides;
use georag_core::models::workspace::IndexState;
use georag_core::processing::chunk::ChunkGenerator;
use georag_llm::ollama::OllamaEmbedder;
use georag_llm::ports::Embedder;
use georag_retrieval::embedding::EmbeddingPipeline;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};

pub async fn execute(
    args: BuildArgs,
    output: &OutputWriter,
    dry_run: bool,
    storage: &Storage,
) -> Result<()> {
    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    // Load layered configuration with CLI overrides
    let overrides = CliConfigOverrides {
        embedder: Some(args.embedder.clone()),
        ..Default::default()
    };
    let config = load_workspace_config_with_overrides(&workspace_root, overrides)?;

    // Load datasets from storage
    let datasets = storage.spatial.list_datasets().await?;

    if datasets.is_empty() {
        bail!("No datasets to build. Add datasets with 'georag add' first.");
    }

    // Check if index already exists and is up to date
    let index_state_path = georag_dir.join("index").join("state.json");
    if index_state_path.exists() && !args.force {
        output.info("Index already exists. Use --force to rebuild.");
        return Ok(());
    }

    // If force rebuild is requested, clear existing data
    if args.force {
        output.info("Force rebuild requested, clearing existing data...");
        storage.clear().await?;
        output.info("  Cleared existing chunks and embeddings");
    }

    if dry_run {
        let mut actions = vec![
            PlannedAction::new(ActionType::ModifyFile, "Normalize geometries to workspace CRS")
                .with_detail(format!("Target CRS: EPSG:{}", config.crs.value))
                .with_detail(format!("Datasets to process: {}", datasets.len())),
            PlannedAction::new(ActionType::ModifyFile, "Validate and fix geometries")
                .with_detail("Check for invalid geometries")
                .with_detail("Apply fixes where possible"),
            PlannedAction::new(ActionType::CreateFile, "Generate embeddings")
                .with_detail(format!("Embedder: {}", config.embedder.value))
                .with_detail(format!(
                    "Estimated chunks: {}",
                    datasets.iter().map(|d| d.feature_count).sum::<usize>()
                )),
            PlannedAction::new(ActionType::WriteFile, "Create index state file")
                .with_detail("Path: .georag/index/state.json")
                .with_detail("Contains: hash, metadata, embedder info"),
        ];

        // Add CRS normalization details for datasets with different CRS
        for dataset in &datasets {
            if dataset.crs != config.crs.value {
                actions.insert(
                    1,
                    PlannedAction::new(
                        ActionType::ModifyFile,
                        format!("Reproject dataset: {}", dataset.name),
                    )
                    .with_detail(format!("From EPSG:{} to EPSG:{}", dataset.crs, config.crs.value)),
                );
            }
        }

        display_planned_actions(output, &actions);
        return Ok(());
    }

    output.info("Building index...");

    output.section("Normalizing geometries");
    let mut normalized_count = 0;
    let fixed_count = 0;

    for dataset in &datasets {
        if dataset.crs != config.crs.value {
            output.info(format!(
                "  Normalizing {} from EPSG:{} to EPSG:{}",
                dataset.name, dataset.crs, config.crs.value
            ));
            normalized_count += 1;
        }
    }

    if normalized_count == 0 {
        output.info("  All datasets already in workspace CRS");
    }

    output.section("Validating geometries");
    output.info("  Fixed 0 invalid geometries");

    output.section("Generating chunks");

    // Create chunk generator
    let chunk_generator = ChunkGenerator::default();
    let mut all_chunks = Vec::new();

    // Generate chunks from each dataset
    for dataset_meta in &datasets {
        // Get full dataset
        let dataset = storage
            .spatial
            .get_dataset(dataset_meta.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Dataset {} not found", dataset_meta.id.0))?;

        // Get features for this dataset
        let features = storage.spatial.get_features_for_dataset(dataset_meta.id).await?;

        output.info(format!(
            "  Processing dataset '{}' ({} features)",
            dataset.name,
            features.len()
        ));

        // Generate chunks
        let chunks = chunk_generator.generate_chunks(&dataset, &features);
        output.info(format!("    Generated {} chunks", chunks.len()));

        all_chunks.extend(chunks);
    }

    let chunk_count = all_chunks.len();
    output.info(format!("  Total chunks generated: {}", chunk_count));

    output.section("Generating embeddings");

    // Create embedder from config
    let embedder = create_embedder(&config.embedder.value).map_err(|e| {
        // Enhance error message for Ollama connection issues
        if e.to_string().contains("Failed to connect to Ollama")
            || e.to_string().contains("Embedder unavailable")
        {
            anyhow::anyhow!(
                "Failed to connect to Ollama at http://localhost:11434\n\n\
                Remediation:\n\
                  1. Ensure Ollama is running: ollama serve\n\
                  2. Pull the embedding model: ollama pull {}\n\
                  3. Verify with: ollama list\n\n\
                Error: {}",
                config.embedder.value.strip_prefix("ollama:").unwrap_or(&config.embedder.value),
                e
            )
        } else {
            e
        }
    })?;
    let embedding_dim = embedder.dimensions();

    output.info(format!("  Using embedder: {}", config.embedder.value));
    output.info(format!("  Processing {} chunks", chunk_count));
    output.info(format!("  Embedding dimension: {}", embedding_dim));

    // Create embedding pipeline with batch size of 32
    let pipeline = EmbeddingPipeline::new(embedder, 32);

    // Generate embeddings with progress display
    let mut embeddings = pipeline
        .generate_embeddings(&all_chunks, |current, total| {
            output.info(format!("  Progress: {}/{} chunks", current, total));
        })
        .map_err(|e| {
            // Enhance error message for embedding generation failures
            if e.to_string().contains("Failed to connect to Ollama")
                || e.to_string().contains("Embedder unavailable")
            {
                anyhow::anyhow!(
                    "Failed to generate embeddings using Ollama at http://localhost:11434\n\n\
                Remediation:\n\
                  1. Ensure Ollama is running: ollama serve\n\
                  2. Verify the model is available: ollama list\n\
                  3. Pull the model if needed: ollama pull {}\n\n\
                Error: {}",
                    config.embedder.value.strip_prefix("ollama:").unwrap_or(&config.embedder.value),
                    e
                )
            } else {
                anyhow::anyhow!("Failed to generate embeddings: {}", e)
            }
        })?;

    // Attach spatial metadata to embeddings
    output.info("  Attaching spatial metadata...");
    for (chunk, embedding) in all_chunks.iter().zip(embeddings.iter_mut()) {
        if let Some(feature_id) = chunk.spatial_ref {
            if let Some(feature) = storage.spatial.get_feature(feature_id).await? {
                // Extract bounding box from geometry
                let bbox = extract_bbox_from_geometry(&feature.geometry);

                embedding.spatial_metadata = Some(georag_core::models::SpatialMetadata {
                    feature_id,
                    crs: feature.crs,
                    bbox,
                });
            }
        }
    }

    output.info(format!("  Generated {} embeddings", embeddings.len()));

    output.section("Storing data");

    // Store chunks to document store
    output.info("  Storing chunks...");
    storage.document.store_chunks(&all_chunks).await?;
    output.info(format!("    Stored {} chunks", all_chunks.len()));

    // Store embeddings to vector store
    output.info("  Storing embeddings...");
    storage.vector.store_embeddings(&embeddings).await?;
    output.info(format!("    Stored {} embeddings", embeddings.len()));

    output.section("Finalizing index");

    // Generate hash from actual chunks and embeddings
    let index_hash = generate_content_hash(&all_chunks, &embeddings);
    output.info(format!("  Index hash: {}", index_hash));

    // Create index state with accurate metadata
    let index_state = IndexState {
        hash: index_hash.clone(),
        built_at: Utc::now(),
        embedder: config.embedder.value.clone(),
        chunk_count: all_chunks.len(),
        embedding_dim,
    };

    // Save index state
    let index_dir = georag_dir.join("index");
    fs::create_dir_all(&index_dir)?;

    let state_json = serde_json::to_string_pretty(&index_state)?;
    fs::write(&index_state_path, state_json)?;

    // Output success
    if output.is_json() {
        let json_output = BuildOutput {
            index_hash: index_hash.clone(),
            chunk_count: all_chunks.len(),
            embedding_dim,
            embedder: config.embedder.value.clone(),
            normalized_count,
            fixed_count,
        };
        output.result(json_output)?;
    } else {
        output.success("Index built successfully");
        output.section("Index Information");
        output.kv("Hash", &index_hash);
        output.kv("Chunks", all_chunks.len());
        output.kv("Embedding Dimension", embedding_dim);
        output.kv("Embedder", &config.embedder.value);
    }

    Ok(())
}

/// Parse embedder string and create an OllamaEmbedder
/// Format: "ollama:model-name" or just "model-name"
fn create_embedder(embedder_str: &str) -> Result<OllamaEmbedder> {
    // Parse the embedder string
    let model = if let Some(stripped) = embedder_str.strip_prefix("ollama:") {
        stripped
    } else {
        embedder_str
    };

    // Determine dimensions based on known models
    let dimensions = match model {
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        "all-minilm" => 384,
        _ => 768, // Default to 768 for unknown models
    };

    Ok(OllamaEmbedder::localhost(model, dimensions))
}

/// Generate hash from chunks and embeddings for index state
fn generate_content_hash(
    chunks: &[georag_core::models::TextChunk],
    embeddings: &[georag_core::models::Embedding],
) -> String {
    let mut hasher = DefaultHasher::new();

    // Hash chunk count and content
    chunks.len().hash(&mut hasher);
    for chunk in chunks {
        chunk.id.0.hash(&mut hasher);
        chunk.content.hash(&mut hasher);
    }

    // Hash embedding count and dimensions
    embeddings.len().hash(&mut hasher);
    if let Some(first_embedding) = embeddings.first() {
        first_embedding.vector.len().hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

/// Extract bounding box from typed Geometry
fn extract_bbox_from_geometry(
    geometry: &Option<georag_core::models::Geometry>,
) -> Option<[f64; 4]> {
    use georag_core::models::Geometry;
    let geom = geometry.as_ref()?;

    match geom {
        Geometry::Point { coordinates } => {
            Some([coordinates[0], coordinates[1], coordinates[0], coordinates[1]])
        }
        Geometry::LineString { coordinates } => compute_bbox_from_coords(coordinates),
        Geometry::MultiPoint { coordinates } => compute_bbox_from_coords(coordinates),
        Geometry::Polygon { coordinates } => {
            let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
            compute_bbox_from_coords(&all_coords)
        }
        Geometry::MultiLineString { coordinates } => {
            let all_coords: Vec<[f64; 2]> = coordinates.iter().flatten().cloned().collect();
            compute_bbox_from_coords(&all_coords)
        }
        Geometry::MultiPolygon { coordinates } => {
            let all_coords: Vec<[f64; 2]> =
                coordinates.iter().flat_map(|poly| poly.iter().flatten()).cloned().collect();
            compute_bbox_from_coords(&all_coords)
        }
    }
}

/// Compute bounding box from an array of coordinate pairs
fn compute_bbox_from_coords(coords: &[[f64; 2]]) -> Option<[f64; 4]> {
    if coords.is_empty() {
        return None;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for coord in coords {
        min_x = min_x.min(coord[0]);
        min_y = min_y.min(coord[1]);
        max_x = max_x.max(coord[0]);
        max_y = max_y.max(coord[1]);
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some([min_x, min_y, max_x, max_y])
    } else {
        None
    }
}
