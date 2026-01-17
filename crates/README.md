# GeoRAG Crates

This directory contains the core Rust crates that make up the GeoRAG workspace.

### georag-core

**Domain models, configuration, and format readers.**

| Module | Purpose |
|--------|---------|
| `config` | Workspace configuration loading |
| `error` | Custom error types (`GeoragError`, `Result`) |
| `formats` | Format readers (GeoJSON, Shapefile, GPX, KML, PDF, DOCX) |
| `models` | Core domain types (Dataset, Feature, Geometry, Chunk) |
| `ports` | Abstract trait definitions |
| `processing` | Text chunking and processing |

```rust
use georag_core::{GeoragError, Result};
use georag_core::models::{Dataset, Feature, Geometry};
use georag_core::formats::FormatRegistry;
```

---

### georag-geo

**Geometry operations, CRS handling, and spatial predicates.**

| Module | Purpose |
|--------|---------|
| `index` | R*-tree spatial indexing |
| `models` | Geometry types, CRS, Distance, SpatialFilter |
| `spatial` | Spatial predicate evaluation (Within, Intersects, DWithin) |
| `transform` | CRS reprojection using PROJ |
| `validation` | Geometry validation and repair |

```rust
use georag_geo::models::{Geometry, Crs, SpatialFilter, SpatialPredicate};
use georag_geo::spatial::evaluate_spatial_filter;
use georag_geo::transform::reproject_geometry;
```

---

### georag-retrieval

**Search pipeline and query execution.**

| Module | Purpose |
|--------|---------|
| `embedding` | Embedding pipeline for index building |
| `index` | Index builder with deterministic hashing |
| `models` | QueryPlan, QueryResult, TextFilter, SourceReference |
| `pipeline` | RetrievalPipeline (spatial → text → semantic) |

```rust
use georag_retrieval::{RetrievalPipeline, QueryPlan, QueryResult};
use georag_retrieval::models::TextFilter;
```

---

### georag-llm

**LLM and embedding integrations.**

| Module | Purpose |
|--------|---------|
| `embedding` | Helper functions for creating embeddings |
| `ollama` | OllamaEmbedder implementation |
| `ports` | Embedder and Generator traits |

```rust
use georag_llm::{OllamaEmbedder, Embedder};

let embedder = OllamaEmbedder::localhost("nomic-embed-text", 768);
let vectors = embedder.embed(&["Hello world"])?;
```

---

### georag-store

**Storage adapters for persistence.**

| Module | Purpose |
|--------|---------|
| `memory` | In-memory storage (MemorySpatialStore, MemoryVectorStore, MemoryDocumentStore) |
| `ports` | Storage traits (SpatialStore, VectorStore, DocumentStore, Transaction) |
| `postgres` | PostgreSQL + PostGIS adapter with migrations |

```rust
use georag_store::memory::MemorySpatialStore;
use georag_store::postgres::{PostgresStore, PostgresConfig};
use georag_store::ports::{SpatialStore, VectorStore};
```

---

### georag-cli

**Command-line interface.**

| Module | Purpose |
|--------|---------|
| `cli` | Clap argument definitions |
| `commands/` | Command implementations (init, add, build, query, etc.) |
| `output` | Output formatting (human, JSON) |
| `batch` | Batch processing utilities |
| `storage` | Storage backend initialization |

```bash
georag init my-workspace
georag add data.geojson
georag build
georag query "search text"
```

---

### georag-api

**HTTP REST API server.**

| Module | Purpose |
|--------|---------|
| `routes` | Axum route handlers |
| `state` | Application state (stores, embedder config) |

```bash
# Start server
DATABASE_URL=postgresql://localhost/georag georag-api

# Query endpoint
curl -X POST http://localhost:3001/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"text": "search", "top_k": 10}'
```

---
