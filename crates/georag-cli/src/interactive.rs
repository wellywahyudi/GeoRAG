use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use std::path::PathBuf;

/// Interactive workspace initialization
pub fn interactive_init() -> Result<InteractiveInitResult> {
    println!("\n🚀 GeoRAG Workspace Setup\n");

    // Workspace path
    let path: String =
        Input::new().with_prompt("Workspace path").default(".".to_string()).interact()?;

    // CRS selection
    let crs_options = vec![
        "4326 - WGS 84 (latitude/longitude) - Most common",
        "3857 - Web Mercator (web maps)",
        "Custom EPSG code",
    ];
    let crs_selection = Select::new()
        .with_prompt("Coordinate Reference System (CRS)")
        .items(&crs_options)
        .default(0)
        .interact()?;

    let crs = match crs_selection {
        0 => 4326,
        1 => 3857,
        2 => {
            let custom: u32 = Input::new().with_prompt("Enter EPSG code").interact()?;
            custom
        }
        _ => 4326,
    };

    // Distance unit
    let distance_units = vec!["Meters", "Kilometers", "Miles", "Feet"];
    let distance_unit_idx = Select::new()
        .with_prompt("Distance unit for spatial operations")
        .items(&distance_units)
        .default(0)
        .interact()?;
    let distance_unit = distance_units[distance_unit_idx].to_lowercase();

    // Geometry validity mode
    let validity_modes = vec![
        "Lenient - Attempt to fix invalid geometries (recommended)",
        "Strict - Reject invalid geometries",
    ];
    let validity_idx = Select::new()
        .with_prompt("Geometry validation mode")
        .items(&validity_modes)
        .default(0)
        .interact()?;
    let validity_mode = if validity_idx == 0 {
        "lenient"
    } else {
        "strict"
    }
    .to_string();

    // Storage backend
    let storage_options = vec![
        "Memory - Fast, no setup (development)",
        "PostgreSQL - Persistent, scalable (production)",
    ];
    let storage_idx = Select::new()
        .with_prompt("Storage backend")
        .items(&storage_options)
        .default(0)
        .interact()?;
    let use_postgres = storage_idx == 1;

    // If PostgreSQL, get connection details
    let database_url = if use_postgres {
        println!("\n📦 PostgreSQL Configuration\n");

        let host: String = Input::new()
            .with_prompt("PostgreSQL host")
            .default("localhost".to_string())
            .interact()?;

        let port: u16 = Input::new().with_prompt("PostgreSQL port").default(5432).interact()?;

        let database: String = Input::new()
            .with_prompt("Database name")
            .default("georag".to_string())
            .interact()?;

        let user: String = Input::new()
            .with_prompt("Username")
            .default("postgres".to_string())
            .interact()?;

        let password: String = Input::new()
            .with_prompt("Password (optional, press Enter to skip)")
            .allow_empty(true)
            .interact()?;

        let url = if password.is_empty() {
            format!("postgresql://{}@{}:{}/{}", user, host, port, database)
        } else {
            format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, database)
        };

        Some(url)
    } else {
        None
    };

    // Confirmation
    println!("\n📋 Configuration Summary\n");
    println!("  Path:           {}", path);
    println!("  CRS:            EPSG:{}", crs);
    println!("  Distance Unit:  {}", distance_unit);
    println!("  Validity Mode:  {}", validity_mode);
    println!("  Storage:        {}", if use_postgres { "PostgreSQL" } else { "Memory" });
    if let Some(ref url) = database_url {
        // Hide password in display
        let display_url = url.split('@').next_back().unwrap_or(url);
        println!("  Database:       {}", display_url);
    }
    println!();

    let confirmed = Confirm::new()
        .with_prompt("Create workspace with these settings?")
        .default(true)
        .interact()?;

    if !confirmed {
        anyhow::bail!("Workspace creation cancelled");
    }

    Ok(InteractiveInitResult {
        path: PathBuf::from(path),
        crs,
        distance_unit,
        validity_mode,
        use_postgres,
        database_url,
    })
}

/// Result from interactive initialization
pub struct InteractiveInitResult {
    pub path: PathBuf,
    pub crs: u32,
    pub distance_unit: String,
    pub validity_mode: String,
    pub use_postgres: bool,
    pub database_url: Option<String>,
}

/// Interactive query builder
#[allow(dead_code)]
pub fn interactive_query() -> Result<InteractiveQueryResult> {
    println!("\n🔍 Query Builder\n");

    // Query text
    let query: String = Input::new().with_prompt("Query text").interact()?;

    // Spatial filter
    let use_spatial =
        Confirm::new().with_prompt("Add spatial filter?").default(false).interact()?;

    let (spatial_predicate, geometry, distance) = if use_spatial {
        // Predicate
        let predicates = vec![
            "within - Features completely inside the geometry",
            "intersects - Features that overlap the geometry",
            "contains - Features that contain the geometry",
            "bbox - Features within bounding box",
        ];
        let predicate_idx = Select::new()
            .with_prompt("Spatial predicate")
            .items(&predicates)
            .default(0)
            .interact()?;

        let predicate = match predicate_idx {
            0 => "within",
            1 => "intersects",
            2 => "contains",
            3 => "bbox",
            _ => "within",
        }
        .to_string();

        // Geometry type
        let geom_types = vec!["Point (latitude, longitude)", "GeoJSON string", "GeoJSON file"];
        let geom_type_idx = Select::new()
            .with_prompt("Geometry type")
            .items(&geom_types)
            .default(0)
            .interact()?;

        let geometry = match geom_type_idx {
            0 => {
                let lat: f64 = Input::new().with_prompt("Latitude").interact()?;
                let lon: f64 = Input::new().with_prompt("Longitude").interact()?;
                format!(r#"{{"type":"Point","coordinates":[{},{}]}}"#, lon, lat)
            }
            1 => Input::new().with_prompt("GeoJSON string").interact()?,
            2 => {
                let path: String = Input::new().with_prompt("GeoJSON file path").interact()?;
                std::fs::read_to_string(path)?
            }
            _ => String::new(),
        };

        // Distance (if applicable)
        let distance = if predicate == "within" {
            let dist: String = Input::new()
                .with_prompt("Distance (e.g., '5km', '100m')")
                .default("1km".to_string())
                .interact()?;
            Some(dist)
        } else {
            None
        };

        (Some(predicate), Some(geometry), distance)
    } else {
        (None, None, None)
    };

    // Number of results
    let top_k: usize = Input::new().with_prompt("Number of results").default(10).interact()?;

    // Semantic reranking
    let no_rerank = !Confirm::new()
        .with_prompt("Enable semantic reranking?")
        .default(true)
        .interact()?;

    Ok(InteractiveQueryResult {
        query,
        spatial_predicate,
        geometry,
        distance,
        top_k,
        no_rerank,
    })
}

/// Result from interactive query
#[allow(dead_code)]
pub struct InteractiveQueryResult {
    pub query: String,
    pub spatial_predicate: Option<String>,
    pub geometry: Option<String>,
    pub distance: Option<String>,
    pub top_k: usize,
    pub no_rerank: bool,
}
