use chrono::{DateTime, Utc};
use georag_core::models::dataset::GeometryType;
use serde::Serialize;

/// Output for init command
#[derive(Debug, Serialize)]
pub struct InitOutput {
    pub workspace_path: String,
    pub crs: u32,
    pub distance_unit: String,
    pub validity_mode: String,
}

/// Output for add command
#[derive(Debug, Serialize)]
pub struct AddOutput {
    pub dataset_name: String,
    pub geometry_type: GeometryType,
    pub feature_count: usize,
    pub crs: u32,
    pub crs_mismatch: Option<CrsMismatchInfo>,
}

#[derive(Debug, Serialize)]
pub struct CrsMismatchInfo {
    pub dataset_crs: u32,
    pub workspace_crs: u32,
}

/// Output for build command
#[derive(Debug, Serialize)]
pub struct BuildOutput {
    pub index_hash: String,
    pub chunk_count: usize,
    pub embedding_dim: usize,
    pub embedder: String,
    pub normalized_count: usize,
    pub fixed_count: usize,
}

/// Output for query command
#[derive(Debug, Serialize)]
pub struct QueryOutput {
    pub query: String,
    pub spatial_matches: usize,
    pub results: Vec<QueryResultItem>,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryResultItem {
    pub content: String,
    pub source: String,
    pub score: Option<f32>,
}

/// Output for inspect datasets command
#[derive(Debug, Serialize)]
pub struct InspectDatasetsOutput {
    pub datasets: Vec<DatasetInfo>,
}

#[derive(Debug, Serialize)]
pub struct DatasetInfo {
    pub id: u64,
    pub name: String,
    pub geometry_type: GeometryType,
    pub feature_count: usize,
    pub crs: u32,
    pub added_at: DateTime<Utc>,
}

/// Output for inspect index command
#[derive(Debug, Serialize)]
pub struct InspectIndexOutput {
    pub built: bool,
    pub hash: Option<String>,
    pub built_at: Option<DateTime<Utc>>,
    pub embedder: Option<String>,
    pub chunk_count: Option<usize>,
    pub embedding_dim: Option<usize>,
}

/// Output for inspect CRS command
#[derive(Debug, Serialize)]
pub struct InspectCrsOutput {
    pub workspace_crs: u32,
    pub datasets: Vec<DatasetCrsInfo>,
}

#[derive(Debug, Serialize)]
pub struct DatasetCrsInfo {
    pub name: String,
    pub crs: u32,
    pub matches_workspace: bool,
}

/// Output for inspect config command
#[derive(Debug, Serialize)]
pub struct InspectConfigOutput {
    pub crs: ConfigValue<u32>,
    pub distance_unit: ConfigValue<String>,
    pub geometry_validity: ConfigValue<String>,
    pub embedder: ConfigValue<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigValue<T> {
    pub value: T,
    pub source: String,
}

/// Output for status command
#[derive(Debug, Serialize)]
pub struct StatusOutput {
    pub workspace_path: String,
    pub crs: u32,
    pub distance_unit: String,
    pub dataset_count: usize,
    pub index: IndexStatus,
    pub storage: Option<StorageStatus>,
}

#[derive(Debug, Serialize)]
pub struct IndexStatus {
    pub built: bool,
    pub hash: Option<String>,
    pub built_at: Option<DateTime<Utc>>,
    pub embedder: Option<String>,
    pub chunk_count: Option<usize>,
    pub embedding_dim: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct StorageStatus {
    pub datasets_dir: bool,
    pub index_dir: bool,
}
