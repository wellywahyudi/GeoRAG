//! Example demonstrating the Ollama embedder
//!
//! This example shows how to use the OllamaEmbedder to generate embeddings
//! and attach spatial metadata.
//!
//! Note: This example requires Ollama to be running locally with a model installed.
//! To run: cargo run --example ollama_example

use georag_llm::{Embedder, OllamaEmbedder, create_embedding_with_spatial_metadata};
use georag_core::models::{ChunkId, FeatureId};

fn main() {
    println!("GeoRAG LLM - Ollama Embedder Example");
    println!("=====================================\n");

    // Create an Ollama embedder
    // Note: This assumes Ollama is running at localhost:11434
    // with the "nomic-embed-text" model installed
    let embedder = OllamaEmbedder::localhost("nomic-embed-text", 768);

    println!("Embedder Configuration:");
    println!("  Model: {}", embedder.model_name());
    println!("  Dimensions: {}", embedder.dimensions());
    println!();

    // Example texts to embed
    let texts = vec![
        "The Golden Gate Bridge is located in San Francisco",
        "Mount Everest is the highest mountain in the world",
    ];

    println!("Attempting to generate embeddings...");
    println!("(This will fail if Ollama is not running)\n");

    // Try to generate embeddings
    match embedder.embed(&texts) {
        Ok(embeddings) => {
            println!("✓ Successfully generated {} embeddings", embeddings.len());
            
            for (i, embedding) in embeddings.iter().enumerate() {
                println!("  Embedding {}: {} dimensions", i + 1, embedding.len());
                println!("    First 5 values: {:?}", &embedding[..5.min(embedding.len())]);
            }
            
            // Demonstrate attaching spatial metadata
            println!("\nAttaching spatial metadata to first embedding:");
            let embedding_with_metadata = create_embedding_with_spatial_metadata(
                ChunkId(1),
                embeddings[0].clone(),
                FeatureId(42),
                4326, // WGS 84
                Some([-122.5, 37.7, -122.4, 37.8]), // San Francisco area bbox
            );
            
            println!("  Chunk ID: {:?}", embedding_with_metadata.chunk_id);
            println!("  Vector dimensions: {}", embedding_with_metadata.vector.len());
            if let Some(metadata) = embedding_with_metadata.spatial_metadata {
                println!("  Spatial metadata:");
                println!("    Feature ID: {:?}", metadata.feature_id);
                println!("    CRS: EPSG:{}", metadata.crs);
                println!("    Bounding box: {:?}", metadata.bbox);
            }
        }
        Err(e) => {
            println!("✗ Failed to generate embeddings:");
            println!("  {}", e);
            println!("\nTo run this example successfully:");
            println!("  1. Install Ollama: https://ollama.ai");
            println!("  2. Start Ollama: ollama serve");
            println!("  3. Pull a model: ollama pull nomic-embed-text");
            println!("  4. Run this example again");
        }
    }
}
