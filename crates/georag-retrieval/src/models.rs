//! Retrieval models

use georag_core::models::{ChunkId, FeatureId, SpatialFilter};
use serde::{Deserialize, Serialize};

/// Query plan with spatial and semantic options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// The text query
    pub text_query: String,
    
    /// Optional spatial filter
    pub spatial_filter: Option<SpatialFilter>,
    
    /// Whether to enable semantic reranking
    pub semantic_rerank: bool,
    
    /// Number of top results to return
    pub top_k: usize,
    
    /// Whether to include detailed explanation
    pub explain: bool,
}

impl QueryPlan {
    /// Create a new query plan
    pub fn new(text_query: impl Into<String>) -> Self {
        Self {
            text_query: text_query.into(),
            spatial_filter: None,
            semantic_rerank: true,
            top_k: 10,
            explain: false,
        }
    }

    /// Set the spatial filter
    pub fn with_spatial_filter(mut self, filter: SpatialFilter) -> Self {
        self.spatial_filter = Some(filter);
        self
    }

    /// Enable or disable semantic reranking
    pub fn with_semantic_rerank(mut self, enabled: bool) -> Self {
        self.semantic_rerank = enabled;
        self
    }

    /// Set the number of top results
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Enable detailed explanation
    pub fn with_explain(mut self, enabled: bool) -> Self {
        self.explain = enabled;
        self
    }
}

/// Query result with answer and sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The generated answer
    pub answer: String,
    
    /// Source references used to ground the answer
    pub sources: Vec<SourceReference>,
    
    /// Number of spatial matches before semantic reranking
    pub spatial_matches: usize,
    
    /// Optional semantic similarity scores
    pub semantic_scores: Option<Vec<f32>>,
    
    /// Optional detailed explanation
    pub explanation: Option<QueryExplanation>,
}

impl QueryResult {
    /// Create a new query result
    pub fn new(answer: impl Into<String>, sources: Vec<SourceReference>, spatial_matches: usize) -> Self {
        Self {
            answer: answer.into(),
            sources,
            spatial_matches,
            semantic_scores: None,
            explanation: None,
        }
    }

    /// Add semantic scores
    pub fn with_semantic_scores(mut self, scores: Vec<f32>) -> Self {
        self.semantic_scores = Some(scores);
        self
    }

    /// Add explanation
    pub fn with_explanation(mut self, explanation: QueryExplanation) -> Self {
        self.explanation = Some(explanation);
        self
    }
}

/// Reference to a source document or feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    /// Chunk ID
    pub chunk_id: ChunkId,
    
    /// Optional feature ID
    pub feature_id: Option<FeatureId>,
    
    /// Source document path
    pub document_path: String,
    
    /// Optional page number
    pub page: Option<usize>,
    
    /// Text excerpt
    pub excerpt: String,
    
    /// Relevance score
    pub score: f32,
}

/// Detailed query explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExplanation {
    /// Spatial phase explanation
    pub spatial_phase: SpatialPhaseExplanation,
    
    /// Optional semantic phase explanation
    pub semantic_phase: Option<SemanticPhaseExplanation>,
    
    /// Ranking details for each result
    pub ranking_details: Vec<RankingDetail>,
}

/// Explanation of the spatial filtering phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialPhaseExplanation {
    /// Spatial predicate used
    pub predicate: String,
    
    /// CRS used for spatial operations
    pub crs: u32,
    
    /// Number of features evaluated
    pub features_evaluated: usize,
    
    /// Number of features that passed the spatial filter
    pub features_matched: usize,
    
    /// Optional distance threshold
    pub distance_threshold: Option<f64>,
}

/// Explanation of the semantic reranking phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticPhaseExplanation {
    /// Embedder model used
    pub embedder_model: String,
    
    /// Embedding dimension
    pub embedding_dim: usize,
    
    /// Number of candidates reranked
    pub candidates_reranked: usize,
    
    /// Query embedding norm
    pub query_norm: f32,
}

/// Ranking detail for a single result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingDetail {
    /// Chunk ID
    pub chunk_id: ChunkId,
    
    /// Spatial score (if applicable)
    pub spatial_score: Option<f32>,
    
    /// Semantic similarity score (if applicable)
    pub semantic_score: Option<f32>,
    
    /// Final combined score
    pub final_score: f32,
    
    /// Explanation of score calculation
    pub score_explanation: String,
}
