//! GeoRAG LLM - Embedding and generation ports
//!
//! This crate defines the ports for embedding and text generation,
//! along with adapter implementations.

pub mod ports;
pub mod ollama;
pub mod embedding;

// Re-export main types
pub use ports::{Embedder, Generator};
pub use ollama::OllamaEmbedder;
pub use embedding::{create_embedding, create_embedding_with_spatial_metadata};
