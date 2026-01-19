# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-20

**Initial Release of GeoRAG** - A local-first, location-aware RAG engine.

### 🚀 Architecture & Core Use Cases
- **Consolidated 5-Crate Architecture**: Clean, modular design (`core`, `store`, `retrieval`, `api`, `cli`).
- **Local-First**: Complete privacy with local Ollama embeddings and no cloud dependencies.
- **Dual Storage**:
  - **In-Memory**: Zero-setup dev mode for fast prototyping.
  - **PostgreSQL + PostGIS**: Robust production storage with vector similarity search (pgvector).

### ✨ Key Features added in v0.1.0

#### 🌍 Geospatial Capabilities
- **Spatial Predicates**: Support for `Within`, `Intersects`, `Contains`, and `DWithin` (Distance Within).
- **Coordinate Systems**: Full CRS support (EPSG codes) with automatic reprojection via PROJ.
- **Indexing**: Fast R*-tree spatial indexing for geometry lookups.
- **Validation**: Strict/Lenient geometry validation modes (fixes self-intersections, unclosed rings).

#### 📄 Data Ingestion
- **Formats**: Native support for GeoJSON, Shapefile, GPX, KML, PDF, and DOCX.
- **Text Chunking**: Smart chunking with overlap and metadata preservation.
- **Metadata**: Automatic extraction of creation dates, authors, and spatial bounds.

#### 🔍 Retrieval Pipeline
- **Hybrid Search**: Combines semantic vector search (cosine similarity) + spatial filtering + text keyword matching.
- **Ollama Integration**: Seamless integration with local LLM models (e.g., `nomic-embed-text`) for embeddings.

#### 🛠 Developer Tools
- **CLI**: Comprehensive command-line tool (`georag`) for managing workspaces, ingesting data, and running queries.
- **REST API**: Production-ready Axum server (`georag-api`) exposing endpoints for query, ingestion, and health checks.
- **Examples**: Included `examples/basic_spatial.rs` and detailed documentation.

### ⚙️ DevOps & CI/CD
- **Automated Releases**: `release-please` workflow for Changelog generation.
- **Cross-Platform Binaries**: Pre-built release assets for Linux, macOS (`amd64`/`arm64`), and Windows.
- **Docker**: (Planned for v0.2.0)

### � Documentation
- **API Reference**: Detailed guide for all HTTP endpoints.
- **CLI Reference**: Usage examples for every command.
- **Community Standards**: Added Code of Conduct and Contributing Guidelines.
