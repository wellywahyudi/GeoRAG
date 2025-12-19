# GeoRAG LLM

This crate provides the embedding and text generation ports for GeoRAG, along with adapter implementations.

## Features

- **Port Interfaces**: Define `Embedder` and `Generator` traits for pluggable LLM backends
- **Ollama Adapter**: Local embedding generation using Ollama
- **Spatial Metadata**: Utilities for attaching geographic context to embeddings

## Usage

### Creating an Ollama Embedder

```rust
use georag_llm::{Embedder, OllamaEmbedder};

// Create embedder with default localhost URL
let embedder = OllamaEmbedder::localhost("nomic-embed-text", 768);

// Or specify a custom URL
let embedder = OllamaEmbedder::new("http://custom:11434", "nomic-embed-text", 768);
```

### Generating Embeddings

```rust
let texts = vec!["San Francisco is a city in California"];
let embeddings = embedder.embed(&texts)?;

println!("Generated {} embeddings", embeddings.len());
println!("Dimensions: {}", embedder.dimensions());
```

### Attaching Spatial Metadata

```rust
use georag_llm::create_embedding_with_spatial_metadata;
use georag_core::models::{ChunkId, FeatureId};

let embedding = create_embedding_with_spatial_metadata(
    ChunkId(1),
    vector,
    FeatureId(42),
    4326, // WGS 84 EPSG code
    Some([-122.5, 37.7, -122.4, 37.8]), // Bounding box
);
```

## Requirements

To use the Ollama embedder:

1. Install Ollama: https://ollama.ai
2. Start the Ollama service: `ollama serve`
3. Pull an embedding model: `ollama pull nomic-embed-text`

## Error Handling

The embedder provides detailed error messages with remediation steps when:

- Ollama is not running
- The specified model is not available
- Network connectivity issues occur

Example error message:

```
Embedder unavailable: Failed to connect to Ollama: connection refused.
Try: Ensure Ollama is running at http://localhost:11434 and the model 'nomic-embed-text'
is available. Run 'ollama pull nomic-embed-text' to download the model.
```

## Architecture

This crate follows hexagonal architecture principles:

- **Ports** (`ports.rs`): Define interfaces for embedding and generation
- **Adapters** (`ollama.rs`): Implement ports for specific backends
- **Utilities** (`embedding.rs`): Helper functions for common operations

The async boundary is managed within adapters, keeping the port interfaces simple and synchronous.
