//! LLM port definitions

use georag_core::error::Result;

/// Port for embedding text into vector representations
pub trait Embedder: Send + Sync {
    /// Generate embeddings for a batch of texts
    ///
    /// # Arguments
    /// * `texts` - Slice of text strings to embed
    ///
    /// # Returns
    /// Vector of embedding vectors, one per input text
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get the dimensionality of embeddings produced by this embedder
    fn dimensions(&self) -> usize;

    /// Get the name/identifier of the embedding model
    fn model_name(&self) -> &str;
}

/// Port for text generation
pub trait Generator: Send + Sync {
    /// Generate text based on a prompt and optional context
    ///
    /// # Arguments
    /// * `prompt` - The generation prompt
    /// * `context` - Optional context strings to ground the generation
    ///
    /// # Returns
    /// Generated text string
    fn generate(&self, prompt: &str, context: &[&str]) -> Result<String>;
}
