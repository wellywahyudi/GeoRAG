pub mod embedding;
pub mod ollama;
pub mod ports;

// Re-export main types
pub use embedding::{create_embedding, create_embedding_with_spatial_metadata};
pub use ollama::OllamaEmbedder;
pub use ports::{Embedder, Generator};
