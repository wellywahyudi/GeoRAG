use georag_core::error::{GeoragError, Result};
use georag_core::models::{ChunkId, ScoredResult, TextChunk};
use georag_llm::ports::Embedder;
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use std::collections::HashMap;

use crate::models::{
    QueryExplanation, QueryPlan, QueryResult, RankingDetail, SemanticPhaseExplanation,
    SourceReference, SpatialPhaseExplanation,
};

/// Retrieval pipeline orchestrating spatial and semantic search
pub struct RetrievalPipeline<S, V, D, E>
where
    S: SpatialStore,
    V: VectorStore,
    D: DocumentStore,
    E: Embedder,
{
    spatial_store: S,
    vector_store: V,
    document_store: D,
    embedder: E,
}

impl<S, V, D, E> RetrievalPipeline<S, V, D, E>
where
    S: SpatialStore,
    V: VectorStore,
    D: DocumentStore,
    E: Embedder,
{
    /// Create a new retrieval pipeline
    pub fn new(spatial_store: S, vector_store: V, document_store: D, embedder: E) -> Self {
        Self {
            spatial_store,
            vector_store,
            document_store,
            embedder,
        }
    }

    /// Execute a query plan
    pub async fn execute(&self, plan: &QueryPlan) -> Result<QueryResult> {
        // Phase 1: Spatial filtering
        let (spatial_candidates, spatial_explanation) = self.spatial_filter_phase(plan).await?;

        // Phase 2: Semantic reranking (if enabled)
        let (ranked_results, semantic_explanation) = if plan.semantic_rerank {
            self.semantic_rerank_phase(plan, &spatial_candidates).await?
        } else {
            // No semantic reranking, just use spatial candidates
            let results: Vec<ScoredResult> = spatial_candidates
                .iter()
                .enumerate()
                .map(|(idx, chunk_id)| ScoredResult {
                    chunk_id: *chunk_id,
                    score: 1.0 - (idx as f32 / spatial_candidates.len() as f32),
                    spatial_score: None,
                })
                .take(plan.top_k)
                .collect();
            (results, None)
        };

        // Phase 3: Result grounding with source references
        let sources = self.ground_results(&ranked_results).await?;

        // Build explanation if requested
        let explanation = if plan.explain {
            let ranking_details = self.build_ranking_details(&ranked_results, &sources).await?;
            Some(QueryExplanation {
                spatial_phase: spatial_explanation.clone(),
                semantic_phase: semantic_explanation.clone(),
                ranking_details,
            })
        } else {
            None
        };

        // Generate answer (placeholder - would use Generator trait in full implementation)
        let answer = self.generate_answer(plan, &sources).await?;

        let semantic_scores = if plan.semantic_rerank {
            Some(ranked_results.iter().map(|r| r.score).collect())
        } else {
            None
        };

        Ok(QueryResult {
            answer,
            sources,
            spatial_matches: spatial_candidates.len(),
            semantic_scores,
            explanation,
        })
    }

    /// Phase 1: Spatial filtering
    async fn spatial_filter_phase(
        &self,
        plan: &QueryPlan,
    ) -> Result<(Vec<ChunkId>, SpatialPhaseExplanation)> {
        let (chunk_ids, features_evaluated, features_matched) = if let Some(filter) =
            &plan.spatial_filter
        {
            // Apply spatial filter
            let features = self.spatial_store.spatial_query(filter).await?;
            let features_matched = features.len();

            // Get all chunks to count features evaluated
            let all_chunk_ids = self.document_store.list_chunk_ids().await?;
            let features_evaluated = all_chunk_ids.len();

            // Extract chunk IDs from features with spatial references
            let chunks = self.document_store.get_chunks(&all_chunk_ids).await?;
            let feature_ids: std::collections::HashSet<_> = features.iter().map(|f| f.id).collect();

            let filtered_chunk_ids: Vec<ChunkId> = chunks
                .into_iter()
                .filter(|chunk| {
                    chunk.spatial_ref.as_ref().map(|fid| feature_ids.contains(fid)).unwrap_or(false)
                })
                .map(|chunk| chunk.id)
                .collect();

            (filtered_chunk_ids, features_evaluated, features_matched)
        } else {
            // No spatial filter, return all chunks
            let chunk_ids = self.document_store.list_chunk_ids().await?;
            let count = chunk_ids.len();
            (chunk_ids, count, count)
        };

        let explanation = SpatialPhaseExplanation {
            predicate: plan
                .spatial_filter
                .as_ref()
                .map(|f| format!("{:?}", f.predicate))
                .unwrap_or_else(|| "None".to_string()),
            crs: plan.spatial_filter.as_ref().map(|f| f.crs).unwrap_or(4326),
            features_evaluated,
            features_matched,
            distance_threshold: plan
                .spatial_filter
                .as_ref()
                .and_then(|f| f.distance.as_ref())
                .map(|d| d.value),
        };

        Ok((chunk_ids, explanation))
    }

    /// Phase 2: Semantic reranking
    async fn semantic_rerank_phase(
        &self,
        plan: &QueryPlan,
        candidates: &[ChunkId],
    ) -> Result<(Vec<ScoredResult>, Option<SemanticPhaseExplanation>)> {
        if candidates.is_empty() {
            return Ok((Vec::new(), None));
        }

        // Generate query embedding
        let query_embeddings = self.embedder.embed(&[&plan.text_query])?;
        let query_embedding = query_embeddings.into_iter().next().ok_or_else(|| {
            GeoragError::EmbedderUnavailable {
                reason: "Failed to generate query embedding".to_string(),
                remediation: "Check embedder configuration".to_string(),
            }
        })?;

        // Calculate query norm
        let query_norm = query_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        // Perform similarity search
        let mut results =
            self.vector_store.similarity_search(&query_embedding, plan.top_k, None).await?;

        // Filter to only include candidates from spatial phase
        let candidate_set: std::collections::HashSet<_> = candidates.iter().copied().collect();
        results.retain(|r| candidate_set.contains(&r.chunk_id));

        // Take top k
        results.truncate(plan.top_k);

        let explanation = SemanticPhaseExplanation {
            embedder_model: self.embedder.model_name().to_string(),
            embedding_dim: self.embedder.dimensions(),
            candidates_reranked: candidates.len(),
            query_norm,
        };

        Ok((results, Some(explanation)))
    }

    /// Phase 3: Ground results with source references
    async fn ground_results(&self, results: &[ScoredResult]) -> Result<Vec<SourceReference>> {
        let chunk_ids: Vec<ChunkId> = results.iter().map(|r| r.chunk_id).collect();
        let chunks = self.document_store.get_chunks(&chunk_ids).await?;

        // Create a map for quick lookup
        let chunk_map: HashMap<ChunkId, TextChunk> =
            chunks.into_iter().map(|c| (c.id, c)).collect();

        let mut sources = Vec::new();
        for result in results {
            if let Some(chunk) = chunk_map.get(&result.chunk_id) {
                sources.push(SourceReference {
                    chunk_id: chunk.id,
                    feature_id: chunk.spatial_ref,
                    document_path: chunk.source.document_path.clone(),
                    page: chunk.source.page,
                    excerpt: chunk.content.clone(),
                    score: result.score,
                });
            }
        }

        Ok(sources)
    }

    /// Build ranking details for explanation
    async fn build_ranking_details(
        &self,
        results: &[ScoredResult],
        sources: &[SourceReference],
    ) -> Result<Vec<RankingDetail>> {
        let mut details = Vec::new();

        for (result, _source) in results.iter().zip(sources.iter()) {
            let score_explanation = if result.spatial_score.is_some() {
                format!(
                    "Combined spatial ({:.3}) and semantic ({:.3}) scores",
                    result.spatial_score.unwrap_or(0.0),
                    result.score
                )
            } else {
                format!("Semantic similarity score: {:.3}", result.score)
            };

            details.push(RankingDetail {
                chunk_id: result.chunk_id,
                spatial_score: result.spatial_score,
                semantic_score: Some(result.score),
                final_score: result.score,
                score_explanation,
            });
        }

        Ok(details)
    }

    /// Generate answer from sources (placeholder implementation)
    async fn generate_answer(
        &self,
        plan: &QueryPlan,
        sources: &[SourceReference],
    ) -> Result<String> {
        // In a full implementation, this would use the Generator trait
        // For now, return a simple concatenation of source excerpts
        if sources.is_empty() {
            return Ok("No relevant information found.".to_string());
        }

        let context: Vec<String> =
            sources.iter().take(3).map(|s| format!("- {}", s.excerpt)).collect();

        Ok(format!(
            "Based on the query '{}', here are the relevant findings:\n\n{}",
            plan.text_query,
            context.join("\n")
        ))
    }
}
