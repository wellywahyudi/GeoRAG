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
