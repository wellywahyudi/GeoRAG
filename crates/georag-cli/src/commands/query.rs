//! Query command implementation

use crate::cli::QueryArgs;
use crate::output::OutputWriter;
use crate::output_types::{QueryOutput, QueryResultItem};
use crate::storage::Storage;
use anyhow::{bail, Context, Result};
use georag_core::models::workspace::IndexState;
use georag_core::models::WorkspaceConfig;
use georag_geo::models::{Distance, DistanceUnit};
use georag_retrieval::models::{QueryPlan, QueryResult, SourceReference};
use std::fs;
use std::path::PathBuf;

pub async fn execute(args: QueryArgs, output: &OutputWriter, explain: bool, _storage: &Storage) -> Result<()> {
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
    
    output.kv("Semantic Reranking", if !args.no_rerank { "Enabled" } else { "Disabled" });
    output.kv("Top K", args.top_k);

    // Simulate query execution (since we don't have actual retrieval implementation yet)
    output.section("Executing Query");
    
    // TODO: Use storage.spatial.spatial_query() for spatial filtering
    // TODO: Use storage.vector.similarity_search() for semantic search
    // TODO: Use storage.document.get_chunks() to retrieve chunk content
    
    // Simulate spatial matches
    let spatial_matches = 5; // Simulated
    output.info(format!("Found {} spatial matches", spatial_matches));

    if !args.no_rerank {
        output.info("Applying semantic reranking...");
    }

    // Create mock results
    let mock_sources = vec![
        SourceReference {
            chunk_id: georag_core::models::ChunkId(1),
            feature_id: Some(georag_core::models::FeatureId(1)),
            document_path: "dataset-1.geojson".to_string(),
            page: None,
            excerpt: "This is a sample text excerpt from the first result...".to_string(),
            score: 0.95,
        },
        SourceReference {
            chunk_id: georag_core::models::ChunkId(2),
            feature_id: Some(georag_core::models::FeatureId(2)),
            document_path: "dataset-1.geojson".to_string(),
            page: None,
            excerpt: "Another relevant excerpt from the second result...".to_string(),
            score: 0.87,
        },
    ];

    let result = QueryResult::new(
        "This is a generated answer based on the retrieved spatial features and documents.",
        mock_sources.clone(),
        spatial_matches,
    );

    // Display results
    if output.is_json() {
        let result_items: Vec<QueryResultItem> = mock_sources
            .iter()
            .map(|s| QueryResultItem {
                content: s.excerpt.clone(),
                source: s.document_path.clone(),
                score: Some(s.score),
            })
            .collect();
        
        let explanation_text = if explain {
            Some(format!(
                "Spatial Phase: {} features evaluated, {} matched. Semantic Phase: Reranked {} candidates using {}",
                spatial_matches + 10, spatial_matches, spatial_matches, index_state.embedder
            ))
        } else {
            None
        };
        
        output.result(QueryOutput {
            query: args.query.clone(),
            spatial_matches,
            results: result_items,
            explanation: explanation_text,
        })?;
    } else {
        output.section("Results");
        output.info(&result.answer);

        output.section("Sources");
        for (i, source) in mock_sources.iter().enumerate() {
            output.info(format!("\n{}. {} (score: {:.2})", i + 1, source.document_path, source.score));
            if let Some(feature_id) = source.feature_id {
                output.kv("  Feature", feature_id.0);
            }
            output.info(format!("  {}", source.excerpt));
        }

        if explain {
            output.section("Explanation");
            output.kv("Spatial Phase", format!("{} features evaluated, {} matched", spatial_matches + 10, spatial_matches));
            output.kv("Semantic Phase", format!("Reranked {} candidates using {}", spatial_matches, index_state.embedder));
            output.kv("Embedding Model", &index_state.embedder);
            output.kv("Embedding Dimension", index_state.embedding_dim);
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

/// Load index state
fn load_index_state(georag_dir: &PathBuf) -> Result<IndexState> {
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
    use georag_core::models::query::{SpatialPredicate, Distance as CoreDistance, DistanceUnit as CoreDistanceUnit};
    
    // Parse predicate
    let predicate = match predicate_str.to_lowercase().as_str() {
        "within" => SpatialPredicate::Within,
        "intersects" => SpatialPredicate::Intersects,
        "contains" => SpatialPredicate::Contains,
        "bbox" | "boundingbox" => SpatialPredicate::BoundingBox,
        _ => bail!("Invalid spatial predicate: {}. Use within, intersects, contains, or bbox", predicate_str),
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

    // Note: Geometry parsing would require more complex logic
    // For now, we'll just create the filter without geometry
    Ok(georag_core::models::SpatialFilter {
        predicate,
        geometry: None,
        distance,
        crs: config.crs,
    })
}

/// Parse distance string like "5km" or "100m"
fn parse_distance(dist_str: &str, default_unit: georag_core::models::workspace::DistanceUnit) -> Result<Distance> {
    let dist_str = dist_str.trim();
    
    // Try to split number and unit
    let (value_str, unit_str) = if let Some(pos) = dist_str.find(|c: char| c.is_alphabetic()) {
        (&dist_str[..pos], &dist_str[pos..])
    } else {
        (dist_str, "")
    };

    let value: f64 = value_str.parse()
        .context("Invalid distance value")?;

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
