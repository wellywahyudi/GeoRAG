use serde::Deserialize;

/// Query request body
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub text: String,
    pub bbox: Option<[f64; 4]>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    10
}

/// Create workspace request body
#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(default = "default_crs")]
    pub crs: u32,
    pub distance_unit: Option<String>,
    pub geometry_validity: Option<String>,
}

fn default_crs() -> u32 {
    4326
}
