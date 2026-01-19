use crate::error::{GeoragError, Result};
use crate::llm::ports::Embedder;
use serde::{Deserialize, Serialize};

/// Ollama embedder implementation
pub struct OllamaEmbedder {
    /// Base URL for Ollama API (e.g., "http://localhost:11434")
    base_url: String,

    /// Model name to use for embeddings
    model: String,

    /// Embedding dimensions (model-specific)
    dimensions: usize,

    /// HTTP client
    client: reqwest::Client,
}

impl OllamaEmbedder {
    /// Create a new Ollama embedder
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dimensions: usize) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            dimensions,
            client: reqwest::Client::new(),
        }
    }

    /// Create with default localhost URL
    pub fn localhost(model: impl Into<String>, dimensions: usize) -> Self {
        Self::new("http://localhost:11434", model, dimensions)
    }
}

impl Embedder for OllamaEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Use tokio runtime to execute async request
        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| GeoragError::EmbedderUnavailable {
                reason: format!("Failed to create async runtime: {}", e),
                remediation: "Ensure tokio is properly configured".to_string(),
            })?;

        runtime.block_on(async {
            let mut embeddings = Vec::with_capacity(texts.len());

            for text in texts {
                let request = OllamaEmbedRequest {
                    model: self.model.clone(),
                    prompt: text.to_string(),
                };

                let response = self
                    .client
                    .post(format!("{}/api/embeddings", self.base_url))
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| GeoragError::EmbedderUnavailable {
                        reason: format!("Failed to connect to Ollama: {}", e),
                        remediation: format!(
                            "Ensure Ollama is running at {} and the model '{}' is available. \
                             Run 'ollama pull {}' to download the model.",
                            self.base_url, self.model, self.model
                        ),
                    })?;

                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(GeoragError::EmbedderUnavailable {
                        reason: format!("Ollama API error ({}): {}", status, error_text),
                        remediation: format!(
                            "Check that the model '{}' is available. Run 'ollama list' to see installed models.",
                            self.model
                        ),
                    });
                }

                let embed_response: OllamaEmbedResponse = response
                    .json()
                    .await
                    .map_err(|e| GeoragError::EmbedderUnavailable {
                        reason: format!("Failed to parse Ollama response: {}", e),
                        remediation: "Check Ollama API compatibility".to_string(),
                    })?;

                embeddings.push(embed_response.embedding);
            }

            Ok(embeddings)
        })
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Request body for Ollama embeddings API
#[derive(Debug, Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

/// Response from Ollama embeddings API
#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_embedder_creation() {
        let embedder = OllamaEmbedder::localhost("nomic-embed-text", 768);
        assert_eq!(embedder.model_name(), "nomic-embed-text");
        assert_eq!(embedder.dimensions(), 768);
    }

    #[test]
    fn test_ollama_embedder_custom_url() {
        let embedder = OllamaEmbedder::new("http://custom:11434", "test-model", 512);
        assert_eq!(embedder.base_url, "http://custom:11434");
        assert_eq!(embedder.model_name(), "test-model");
        assert_eq!(embedder.dimensions(), 512);
    }
}
