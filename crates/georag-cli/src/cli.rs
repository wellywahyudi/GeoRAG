//! CLI argument parsing

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// GeoRAG - Geospatial retrieval-augmented system
#[derive(Parser, Debug)]
#[command(name = "georag")]
#[command(about = "Geospatial retrieval-augmented system", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Output results in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Show planned actions without executing them
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Show detailed explanation of operations
    #[arg(long, global = true)]
    pub explain: bool,

    /// Storage backend to use (memory or postgres)
    #[arg(long, global = true, default_value = "memory")]
    pub storage: StorageBackend,

    #[command(subcommand)]
    pub command: Commands,
}

/// Storage backend selection
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum StorageBackend {
    /// In-memory storage (default, for development)
    Memory,
    /// PostgreSQL persistent storage
    Postgres,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new GeoRAG workspace
    Init(InitArgs),

    /// Add a dataset to the workspace
    Add(AddArgs),

    /// Build the retrieval index
    Build(BuildArgs),

    /// Query the index
    Query(QueryArgs),

    /// Inspect workspace state
    Inspect(InspectArgs),

    /// Show workspace status
    Status(StatusArgs),

    /// Migrate data from in-memory storage to PostgreSQL
    Migrate(MigrateArgs),

    /// Manage database indexes
    Index(IndexArgs),
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Workspace directory path (defaults to current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// CRS EPSG code (e.g., 4326 for WGS 84)
    #[arg(long, default_value = "4326")]
    pub crs: u32,

    /// Distance unit for spatial operations
    #[arg(long, default_value = "meters")]
    pub distance_unit: String,

    /// Geometry validity mode (strict or lenient)
    #[arg(long, default_value = "lenient")]
    pub validity_mode: String,

    /// Force overwrite if workspace already exists
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct AddArgs {
    /// Path to the dataset file (GeoJSON, Shapefile, etc.)
    pub path: PathBuf,

    /// Dataset name (defaults to filename)
    #[arg(long)]
    pub name: Option<String>,

    /// Override CRS mismatch warning
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Embedder to use (e.g., "ollama:nomic-embed-text")
    #[arg(long, default_value = "ollama:nomic-embed-text")]
    pub embedder: String,

    /// Force rebuild even if index is up to date
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct QueryArgs {
    /// The query text
    pub query: String,

    /// Spatial filter predicate (within, intersects, contains, bbox)
    #[arg(long)]
    pub spatial: Option<String>,

    /// Filter geometry (GeoJSON string or file path)
    #[arg(long)]
    pub geometry: Option<String>,

    /// Distance for proximity queries (e.g., "5km", "100m")
    #[arg(long)]
    pub distance: Option<String>,

    /// Disable semantic reranking
    #[arg(long)]
    pub no_rerank: bool,

    /// Number of results to return
    #[arg(long, short = 'k', default_value = "10")]
    pub top_k: usize,
}

#[derive(Parser, Debug)]
pub struct InspectArgs {
    /// What to inspect
    #[command(subcommand)]
    pub target: InspectTarget,
}

#[derive(Subcommand, Debug)]
pub enum InspectTarget {
    /// Inspect registered datasets
    Datasets,

    /// Inspect the index
    Index,

    /// Inspect CRS information
    Crs,

    /// Inspect configuration
    Config,
}

#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Show detailed status
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    /// PostgreSQL database URL (e.g., postgresql://user:pass@localhost/georag)
    #[arg(long)]
    pub database_url: String,

    /// Show what would be migrated without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Batch size for transferring records
    #[arg(long, default_value = "1000")]
    pub batch_size: usize,

    /// Verify data integrity after migration
    #[arg(long)]
    pub verify: bool,
}

#[derive(Parser, Debug)]
pub struct IndexArgs {
    /// Index management command
    #[command(subcommand)]
    pub command: IndexCommand,
}

#[derive(Subcommand, Debug)]
pub enum IndexCommand {
    /// Rebuild database indexes
    Rebuild(RebuildArgs),

    /// Show index statistics
    Stats(StatsArgs),

    /// Run VACUUM and ANALYZE for maintenance
    Vacuum(VacuumArgs),
}

#[derive(Parser, Debug)]
pub struct RebuildArgs {
    /// Specific index to rebuild (rebuilds all if not specified)
    #[arg(long)]
    pub index: Option<String>,

    /// Rebuild indexes concurrently (non-blocking)
    #[arg(long, default_value = "true")]
    pub concurrently: bool,
}

#[derive(Parser, Debug)]
pub struct StatsArgs {
    /// Specific index to show stats for (shows all if not specified)
    #[arg(long)]
    pub index: Option<String>,
}

#[derive(Parser, Debug)]
pub struct VacuumArgs {
    /// Specific table to vacuum (vacuums all if not specified)
    #[arg(long)]
    pub table: Option<String>,

    /// Run ANALYZE after VACUUM
    #[arg(long, default_value = "true")]
    pub analyze: bool,

    /// Run FULL vacuum (locks table, reclaims more space)
    #[arg(long)]
    pub full: bool,
}
