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

/// Extended dataset information for workspace-scoped responses
#[derive(Debug, Serialize)]
pub struct DatasetResponse {
    pub id: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub feature_count: usize,
    pub crs: u32,
    pub added_at: DateTime<Utc>,
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

/// Workspace response
#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub crs: u32,
    pub distance_unit: String,
    pub geometry_validity: String,
    pub created_at: DateTime<Utc>,
}

/// Index status response for workspace-scoped index operations
#[derive(Debug, Serialize)]
pub struct IndexStatusResponse {
    pub built: bool,
    pub rebuilding: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedder: Option<String>,
}

/// Rebuild operation response (202 Accepted)
#[derive(Debug, Serialize)]
pub struct RebuildResponse {
    pub status: String,
    pub message: String,
}

impl RebuildResponse {
    pub fn accepted() -> Self {
        Self {
            status: "accepted".to_string(),
            message: "Index rebuild started. Poll GET /index/status for progress.".to_string(),
        }
    }
}

/// Delete operation response
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

impl DeleteResponse {
    pub fn success(entity: &str, id: &str) -> Self {
        Self {
            success: true,
            message: format!("Successfully deleted {} {}", entity, id),
        }
    }
}

