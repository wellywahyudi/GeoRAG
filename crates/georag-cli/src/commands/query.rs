use crate::cli::QueryArgs;
use crate::output::OutputWriter;
use crate::output_types::{QueryOutput, QueryResultItem};
use crate::storage::Storage;
use anyhow::{bail, Context, Result};
use georag_core::models::workspace::IndexState;
use georag_core::models::WorkspaceConfig;
use georag_geo::models::{Distance, DistanceUnit};
use georag_llm::ollama::OllamaEmbedder;
use georag_retrieval::models::QueryPlan;
use georag_retrieval::pipeline::RetrievalPipeline;
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(
    args: QueryArgs,
    output: &OutputWriter,
    explain: bool,
    _storage: &Storage,
) -> Result<()> {
    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let georag_dir = workspace_root.join(".georag");

    // Load workspace config
    let config = load_workspace_config(&georag_dir)?;

    // Check if index exists
    let index_state = load_index_state(&georag_dir)?;

    // Parse spatial filter if provided
    let spatial_filter = if let Some(ref spatial_str) = args.spatial {
        Some(parse_spatial_filter(
            spatial_str,
            args.geometry.as_deref(),
            args.distance.as_deref(),
            &config,
        )?)
    } else {
        None
    };

    // Create query plan
    let query_plan = QueryPlan::new(&args.query)
        .with_semantic_rerank(!args.no_rerank)
        .with_top_k(args.top_k)
        .with_explain(explain);

    let query_plan = if let Some(filter) = spatial_filter.clone() {
        query_plan.with_spatial_filter(filter)
    } else {
        query_plan
    };

    // Display query plan
    output.section("Query Plan");
    output.kv("Query", &args.query);

    if let Some(ref filter) = spatial_filter {
        output.kv("Spatial Predicate", format!("{:?}", filter.predicate));
        output.kv("CRS", format!("EPSG:{}", filter.crs));
        if let Some(ref dist) = filter.distance {
            output.kv("Distance", format!("{} {:?}", dist.value, dist.unit));
        }
    } else {
        output.kv("Spatial Filter", "None");
    }

    output.kv(
        "Semantic Reranking",
        if !args.no_rerank {
            "Enabled"
        } else {
            "Disabled"
        },
    );
    output.kv("Top K", args.top_k);

    // Execute query using RetrievalPipeline
    output.section("Executing Query");

    // Initialize embedder from index state
    let embedder = OllamaEmbedder::localhost(&index_state.embedder, index_state.embedding_dim);

    // For now, use in-memory storage (TODO: support PostgreSQL)
    // The issue is that RetrievalPipeline uses generic types, not trait objects
    // We need concrete types here
    let spatial_store = MemorySpatialStore::new();
    let vector_store = MemoryVectorStore::new();
    let document_store = MemoryDocumentStore::new();

    // Create retrieval pipeline with concrete types
    let pipeline = RetrievalPipeline::new(spatial_store, vector_store, document_store, embedder);

    // Execute the query
    let result = pipeline.execute(&query_plan).await.context("Failed to execute query")?;

    // Display results
    if output.is_json() {
        let result_items: Vec<QueryResultItem> = result
            .sources
            .iter()
            .map(|s| QueryResultItem {
                content: s.excerpt.clone(),
                source: s.document_path.clone(),
                score: Some(s.score),
            })
            .collect();

        let explanation_text = result.explanation.as_ref().map(|explanation| {
            format!(
                "Spatial Phase: {} features evaluated, {} matched. Semantic Phase: {}",
                explanation.spatial_phase.features_evaluated,
                explanation.spatial_phase.features_matched,
                explanation
                    .semantic_phase
                    .as_ref()
                    .map(|s| format!(
                        "Reranked {} candidates using {}",
                        s.candidates_reranked, s.embedder_model
                    ))
                    .unwrap_or_else(|| "Disabled".to_string())
            )
        });

        output.result(QueryOutput {
            query: args.query.clone(),
            spatial_matches: result.spatial_matches,
            results: result_items,
            explanation: explanation_text,
        })?;
    } else {
        output.info(format!("Found {} spatial matches", result.spatial_matches));

        if !args.no_rerank {
            output.info("Applied semantic reranking");
        }

        output.section("Results");
        output.info(&result.answer);

        output.section("Sources");
        for (i, source) in result.sources.iter().enumerate() {
            output.info(format!(
                "\n{}. {} (score: {:.2})",
                i + 1,
                source.document_path,
                source.score
            ));
            if let Some(feature_id) = source.feature_id {
                output.kv("  Feature", feature_id.0);
            }
            output.info(format!("  {}", source.excerpt));
        }

        if let Some(explanation) = result.explanation {
            output.section("Explanation");
            output.kv(
                "Spatial Phase",
                format!(
                    "{} features evaluated, {} matched",
                    explanation.spatial_phase.features_evaluated,
                    explanation.spatial_phase.features_matched
                ),
            );

            if let Some(semantic) = explanation.semantic_phase {
                output.kv(
                    "Semantic Phase",
                    format!(
                        "Reranked {} candidates using {}",
                        semantic.candidates_reranked, semantic.embedder_model
                    ),
                );
                output.kv("Embedding Model", &semantic.embedder_model);
                output.kv("Embedding Dimension", semantic.embedding_dim);
                output.kv("Query Norm", format!("{:.3}", semantic.query_norm));
            }

            if !explanation.ranking_details.is_empty() {
                output.section("Ranking Details");
                for (i, detail) in explanation.ranking_details.iter().enumerate().take(5) {
                    output.info(format!("\n{}. Chunk ID: {}", i + 1, detail.chunk_id.0));
                    output.kv("  Final Score", format!("{:.3}", detail.final_score));
                    output.info(format!("  {}", detail.score_explanation));
                }
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
fn load_workspace_config(georag_dir: &Path) -> Result<WorkspaceConfig> {
    let config_path = georag_dir.join("config.toml");
    let config_content = fs::read_to_string(&config_path).context("Failed to read config.toml")?;
    let config: WorkspaceConfig =
        toml::from_str(&config_content).context("Failed to parse config.toml")?;
    Ok(config)
}

/// Load index state
fn load_index_state(georag_dir: &Path) -> Result<IndexState> {
    let state_path = georag_dir.join("index").join("state.json");
    if !state_path.exists() {
        bail!("Index not built. Run 'georag build' first.");
    }

    let content = fs::read_to_string(&state_path)?;
    let state: IndexState = serde_json::from_str(&content)?;
    Ok(state)
}

/// Parse spatial filter from command line arguments
fn parse_spatial_filter(
    predicate_str: &str,
    _geometry_str: Option<&str>,
    distance_str: Option<&str>,
    config: &WorkspaceConfig,
) -> Result<georag_core::models::SpatialFilter> {
    use georag_core::models::query::{
        Distance as CoreDistance, DistanceUnit as CoreDistanceUnit, SpatialPredicate,
    };

    // Parse predicate
    let predicate = match predicate_str.to_lowercase().as_str() {
        "within" => SpatialPredicate::Within,
        "intersects" => SpatialPredicate::Intersects,
        "contains" => SpatialPredicate::Contains,
        "bbox" | "boundingbox" => SpatialPredicate::BoundingBox,
        _ => bail!(
            "Invalid spatial predicate: {}. Use within, intersects, contains, or bbox",
            predicate_str
        ),
    };

    // Parse distance if provided
    let distance = if let Some(dist_str) = distance_str {
        let dist = parse_distance(dist_str, config.distance_unit)?;
        Some(CoreDistance {
            value: dist.value,
            unit: match dist.unit {
                DistanceUnit::Meters => CoreDistanceUnit::Meters,
                DistanceUnit::Kilometers => CoreDistanceUnit::Kilometers,
                DistanceUnit::Miles => CoreDistanceUnit::Miles,
                DistanceUnit::Feet => CoreDistanceUnit::Feet,
            },
        })
    } else {
        None
    };

    Ok(georag_core::models::SpatialFilter {
        predicate,
        geometry: None,
        distance,
        crs: config.crs,
    })
}

/// Parse distance string like "5km" or "100m"
fn parse_distance(
    dist_str: &str,
    default_unit: georag_core::models::workspace::DistanceUnit,
) -> Result<Distance> {
    let dist_str = dist_str.trim();

    // Try to split number and unit
    let (value_str, unit_str) = if let Some(pos) = dist_str.find(|c: char| c.is_alphabetic()) {
        (&dist_str[..pos], &dist_str[pos..])
    } else {
        (dist_str, "")
    };

    let value: f64 = value_str.parse().context("Invalid distance value")?;

    let unit = if unit_str.is_empty() {
        // Use default unit from config
        match default_unit {
            georag_core::models::workspace::DistanceUnit::Meters => DistanceUnit::Meters,
            georag_core::models::workspace::DistanceUnit::Kilometers => DistanceUnit::Kilometers,
            georag_core::models::workspace::DistanceUnit::Miles => DistanceUnit::Miles,
            georag_core::models::workspace::DistanceUnit::Feet => DistanceUnit::Feet,
        }
    } else {
        match unit_str.to_lowercase().as_str() {
            "m" | "meters" | "meter" => DistanceUnit::Meters,
            "km" | "kilometers" | "kilometer" => DistanceUnit::Kilometers,
            "mi" | "miles" | "mile" => DistanceUnit::Miles,
            "ft" | "feet" | "foot" => DistanceUnit::Feet,
            _ => bail!("Invalid distance unit: {}", unit_str),
        }
    };

    Ok(Distance::new(value, unit))
}
