use crate::cli::BuildArgs;
use crate::config::{find_workspace_root, load_workspace_config_with_overrides};
use crate::dry_run::{display_planned_actions, ActionType, PlannedAction};
use crate::output::OutputWriter;
use crate::output_types::BuildOutput;
use crate::storage::Storage;
use anyhow::{bail, Result};
use georag_core::config::CliConfigOverrides;
use georag_geo::models::Crs;
use georag_llm::ollama::OllamaEmbedder;
use georag_retrieval::{IndexBuilder, IndexPhase, IndexProgress};
use std::fs;

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

    // Create embedder from config
    let embedder = create_embedder(&config.embedder.value).map_err(|e| {
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

    // Create workspace CRS
    let workspace_crs = Crs::new(config.crs.value, format!("EPSG:{}", config.crs.value));

    // Create IndexBuilder using the shared service
    let builder = IndexBuilder::new(
        storage.spatial.clone(),
        storage.vector.clone(),
        storage.document.clone(),
        embedder,
        workspace_crs,
    )
    .with_batch_size(32);

    // Track state for output
    let mut last_phase = IndexPhase::Initializing;

    // Perform full rebuild with progress display
    let result = builder
        .full_rebuild(&datasets, args.force, |progress: IndexProgress| {
            // Only print section headers when phase changes
            if progress.phase != last_phase {
                match progress.phase {
                    IndexPhase::Initializing => output.section("Initializing"),
                    IndexPhase::GeneratingChunks => output.section("Generating chunks"),
                    IndexPhase::GeneratingEmbeddings => output.section("Generating embeddings"),
                    IndexPhase::StoringData => output.section("Storing data"),
                    IndexPhase::Finalizing => output.section("Finalizing index"),
                }
                last_phase = progress.phase;
            }
            output.info(format!("  {}", progress.message));
        })
        .await
        .map_err(|e| {
            if e.to_string().contains("Failed to connect to Ollama")
                || e.to_string().contains("Embedder unavailable")
            {
                anyhow::anyhow!(
                    "Failed to generate embeddings using Ollama\n\n\
                    Remediation:\n\
                      1. Ensure Ollama is running: ollama serve\n\
                      2. Verify the model is available: ollama list\n\
                      3. Pull the model if needed: ollama pull {}\n\n\
                    Error: {}",
                    config.embedder.value.strip_prefix("ollama:").unwrap_or(&config.embedder.value),
                    e
                )
            } else {
                anyhow::anyhow!("Failed to build index: {}", e)
            }
        })?;

    // Create index state
    let index_state = builder.create_index_state(&result);

    // Save index state to disk
    let index_dir = georag_dir.join("index");
    fs::create_dir_all(&index_dir)?;

    let state_json = serde_json::to_string_pretty(&index_state)?;
    fs::write(&index_state_path, state_json)?;

    // Output success
    if output.is_json() {
        let json_output = BuildOutput {
            index_hash: result.index_hash.clone(),
            chunk_count: result.chunk_count,
            embedding_dim: result.embedding_dim,
            embedder: config.embedder.value.clone(),
            normalized_count: result.geometries_normalized,
            fixed_count: result.geometries_fixed,
        };
        output.result(json_output)?;
    } else {
        output.success("Index built successfully");
        output.section("Index Information");
        output.kv("Hash", &result.index_hash);
        output.kv("Chunks", result.chunk_count);
        output.kv("Embedding Dimension", result.embedding_dim);
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
