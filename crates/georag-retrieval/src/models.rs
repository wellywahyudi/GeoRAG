use georag_core::models::{ChunkId, FeatureId, SpatialFilter};
use serde::{Deserialize, Serialize};

/// Text filter for keyword-based filtering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextFilter {
    /// Keywords that appear in the text
    pub must_contain: Vec<String>,

    /// Keywords that do not appear in the text
    pub must_not_contain: Vec<String>,

    /// Whether matching is case-sensitive (default: false)
    #[serde(default)]
    pub case_sensitive: bool,
}

impl TextFilter {
    /// Create a new empty text filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a keyword that must be present
    pub fn must(mut self, keyword: impl Into<String>) -> Self {
        self.must_contain.push(keyword.into());
        self
    }

    /// Add multiple keywords that must be present
    pub fn must_all(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.must_contain.extend(keywords.into_iter().map(Into::into));
        self
    }

    /// Add a keyword that does not appear in the text
    pub fn must_not(mut self, keyword: impl Into<String>) -> Self {
        self.must_not_contain.push(keyword.into());
        self
    }

    /// Add multiple keywords that do not appear in the text
    pub fn must_not_any(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.must_not_contain.extend(keywords.into_iter().map(Into::into));
        self
    }

    /// Set case sensitivity
    pub fn with_case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Check if the filter is empty (no constraints)
    pub fn is_empty(&self) -> bool {
        self.must_contain.is_empty() && self.must_not_contain.is_empty()
    }

    /// Check if text matches this filter
    pub fn matches(&self, text: &str) -> bool {
        let text_normalized = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        // Check must_contain (all must be present)
        for keyword in &self.must_contain {
            let kw = if self.case_sensitive {
                keyword.clone()
            } else {
                keyword.to_lowercase()
            };
            if !text_normalized.contains(&kw) {
                return false;
            }
        }

        // Check must_not_contain (none can be present)
        for keyword in &self.must_not_contain {
            let kw = if self.case_sensitive {
                keyword.clone()
            } else {
                keyword.to_lowercase()
            };
            if text_normalized.contains(&kw) {
                return false;
            }
        }

        true
    }
}

/// Query plan with spatial, text, and semantic options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// The text query for semantic similarity
    pub text_query: String,

    /// Optional spatial filter
    pub spatial_filter: Option<SpatialFilter>,

    /// Optional text filter for keyword matching
    pub text_filter: Option<TextFilter>,

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
            text_filter: None,
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

    /// Set the text filter
    pub fn with_text_filter(mut self, filter: TextFilter) -> Self {
        self.text_filter = Some(filter);
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
    pub fn new(
        answer: impl Into<String>,
        sources: Vec<SourceReference>,
        spatial_matches: usize,
    ) -> Self {
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
