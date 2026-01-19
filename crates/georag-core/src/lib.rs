pub mod config;
pub mod error;
pub mod formats;
pub mod geo;
pub mod llm;
pub mod models;
pub mod processing;

pub use error::{GeoragError, Result};
pub use llm::{Embedder, Generator, OllamaEmbedder};
