use chrono::{DateTime, Utc};
use serde::Serialize;

/// Dataset information response
#[derive(Debug, Serialize)]
pub struct DatasetInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub count: usize,
}

/// Ingest operation response
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_id: Option<u64>,
    pub message: String,
}

impl IngestResponse {
    pub fn success(dataset_id: u64, filename: &str, feature_count: usize) -> Self {
        Self {
            success: true,
            dataset_id: Some(dataset_id),
            message: format!("Successfully ingested {} with {} features", filename, feature_count),
        }
    }
}

/// Index integrity response
#[derive(Debug, Serialize)]
pub struct IndexIntegrityResponse {
    pub hash: String,
    pub built_at: DateTime<Utc>,
    pub embedder: String,
    pub chunk_count: usize,
    pub embedding_dim: usize,
}

/// Index verification response
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub stored_hash: String,
    pub computed_hash: String,
    pub matches: bool,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

impl Default for HealthResponse {
    fn default() -> Self {
        Self { status: "ok", service: "georag-api" }
    }
}
