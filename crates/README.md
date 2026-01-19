# GeoRAG Crates

This directory contains the core Rust crates that make up the GeoRAG workspace.

## Architecture Overview

```
georag/
├── crates/
│   ├── georag-core    # Domain models, errors, formats, geo, llm traits
│   ├── georag-store   # Storage adapters (Memory, PostgreSQL) + port traits
│   ├── georag-retrieval # Search pipeline, query execution
│   ├── georag-api     # REST API (Axum)
│   └── georag-cli     # CLI application (Clap)
```

---

### georag-core

**Domain models, configuration, format readers, geo operations, and LLM traits.**

| Module | Purpose |
|--------|---------|
| `config` | Workspace configuration loading |
| `error` | Custom error types (`GeoragError`, `Result`) |
| `formats` | Format readers (GeoJSON, Shapefile, GPX, KML, PDF, DOCX) |
| `geo` | Geometry operations, CRS transforms, spatial indexing, validation |
| `llm` | LLM/embedding traits and Ollama implementation |
| `models` | Core domain types (Dataset, Feature, Geometry, Chunk) |
| `processing` | Text chunking and processing |

```rust
use georag_core::{GeoragError, Result};
use georag_core::models::{Dataset, Feature, Geometry};
use georag_core::formats::FormatRegistry;
use georag_core::geo::models::{Crs, SpatialFilter, SpatialPredicate};
use georag_core::geo::spatial::evaluate_spatial_filter;
use georag_core::llm::{Embedder, OllamaEmbedder};
```

---

### georag-store

**Storage adapters for persistence.**

| Module | Purpose |
|--------|---------|
| `memory` | In-memory storage (MemorySpatialStore, MemoryVectorStore, MemoryDocumentStore) |
| `ports` | Storage traits (SpatialStore, VectorStore, DocumentStore, WorkspaceStore, Transaction) |
| `postgres` | PostgreSQL + PostGIS adapter with migrations |

```rust
use georag_store::memory::MemorySpatialStore;
use georag_store::postgres::{PostgresStore, PostgresConfig};
use georag_store::ports::{SpatialStore, VectorStore, DocumentStore};
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
| `handlers` | Request handlers for workspaces, datasets, queries |
| `services` | Business logic for ingestion and querying |

```bash
# Start server
DATABASE_URL=postgresql://localhost/georag georag-api

# Query endpoint
curl -X POST http://localhost:3001/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"text": "search", "top_k": 10}'
```

---

## Dependency Graph

```
                    ┌─────────────────┐
                    │   georag-core   │
                    │ (models, geo,   │
                    │  llm, formats)  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
       ┌────────────┐ ┌────────────┐ ┌────────────┐
       │georag-store│ │            │ │            │
       │  (ports,   │ │            │ │            │
       │  adapters) │ │            │ │            │
       └─────┬──────┘ │            │ │            │
             │        │            │ │            │
             └────────┼────────────┘ │            │
                      │              │            │
                      ▼              │            │
              ┌────────────────┐     │            │
              │georag-retrieval│     │            │
              │   (pipeline)   │     │            │
              └───────┬────────┘     │            │
                      │              │            │
        ┌─────────────┴──────────────┘            │
        │                                         │
        ▼                                         ▼
 ┌────────────┐                            ┌────────────┐
 │ georag-api │                            │ georag-cli │
 │   (REST)   │                            │   (CLI)    │
 └────────────┘                            └────────────┘
```
