# GeoRAG Documentation

**Geospatial Retrieval-Augmented Generation** - Combine spatial data with semantic search.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Basic Workflow](#basic-workflow)
- [Core Concepts](#core-concepts)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Reference](#reference)

---

## Quick Start

```bash
# 1. Install
cargo install --path crates/georag-cli

# 2. Start Ollama
ollama pull nomic-embed-text

# 3. Initialize workspace
georag init my-project
cd my-project

# 4. Add datasets
georag add cities.geojson

# 5. Build index
georag build

# 6. Query
georag query "What cities are in the dataset?"
```

---

## Installation

### Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| Rust | 1.70+ | Build toolchain |
| Ollama | Latest | Embedding generation |
| PostgreSQL | 14+ | Optional persistent storage |

### Build from Source

```bash
git clone https://github.com/wellywahyudi/georag.git
cd georag

# Build all crates
cargo build --release

# Install CLI
cargo install --path crates/georag-cli

# Verify
georag --version
```

### Start Dependencies

```bash
# Pull embedding model
ollama pull nomic-embed-text

# Verify Ollama
ollama list

# Optional: Start PostgreSQL
docker run -d --name georag-db \
  -e POSTGRES_DB=georag \
  -e POSTGRES_PASSWORD=secret \
  -p 5432:5432 postgres:14
```

---

## Basic Workflow

### Step 1: Initialize Workspace

```bash
georag init my-project
cd my-project
```

This creates:
```
my-project/
└── .georag/
    ├── config.toml      # Workspace configuration
    ├── datasets/        # Stored datasets
    └── index/           # Built index
```

### Step 2: Add Datasets

**Supported Formats:**

| Format | Extensions | Description |
|--------|------------|-------------|
| GeoJSON | `.geojson` | Standard geospatial JSON |
| Shapefile | `.shp` | ESRI Shapefile |
| GPX | `.gpx` | GPS tracks/waypoints |
| KML | `.kml` | Google Earth format |
| PDF | `.pdf` | Documents (with geometry) |
| DOCX | `.docx` | Word documents |

```bash
# Single file
georag add cities.geojson

# Batch directory
georag add data/

# With options
georag add report.pdf --geometry location.geojson
```

### Step 3: Build Index

```bash
georag build

# With specific embedder
georag build --embedder ollama:mxbai-embed-large

# Force rebuild
georag build --force
```

### Step 4: Query

```bash
# Basic query
georag query "What features exist?"

# Spatial query
georag query "What's nearby?" \
  --spatial dwithin \
  --geometry point.geojson \
  --distance 5km

# Text filtering
georag query "Find restaurants" \
  --must-contain "seafood" \
  --exclude "closed"
```

### Step 5: Check Status

```bash
georag status
georag doctor
```

---

## Core Concepts

### Workspace

A workspace is a directory containing:

| Path | Purpose |
|------|---------|
| `.georag/config.toml` | CRS, units, validation settings |
| `.georag/datasets/` | Registered dataset files |
| `.georag/index/` | Built retrieval index |

### Coordinate Reference Systems (CRS)

- Every workspace has a defined CRS (default: EPSG:4326)
- Datasets are validated for CRS compatibility
- Use `--force` to override CRS mismatch warnings

### Retrieval Pipeline

```
Query → Spatial Filter → Text Filter → Semantic Ranking → Results
         ↓                ↓              ↓
      BBox/DWithin    Must/Exclude   Embeddings
```

### Spatial Predicates

| Predicate | Description |
|-----------|-------------|
| `within` | Feature is inside filter geometry |
| `intersects` | Feature overlaps filter geometry |
| `contains` | Feature contains filter geometry |
| `bbox` | Bounding boxes intersect (fast) |
| `dwithin` | Within geodesic distance |

---

## Architecture

```
┌─────────────────────────────────────────┐
│           CLI / REST API                │
│      (georag-cli, georag-api)           │
└────────────────┬────────────────────────┘
                 │
┌────────────────┴────────────────────────┐
│         Retrieval Pipeline              │
│          (georag-retrieval)             │
└────────────────┬────────────────────────┘
                 │
┌────────┬───────┴───────┬────────────────┐
│ georag │   georag-geo  │  georag-llm    │
│ -core  │   (spatial)   │  (embeddings)  │
└────────┴───────┬───────┴────────────────┘
                 │
┌────────────────┴────────────────────────┐
│            Storage Layer                │
│   (Memory / PostgreSQL adapters)        │
└─────────────────────────────────────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| `georag-core` | Domain models, configuration, format readers |
| `georag-geo` | Geometry operations, CRS, spatial predicates |
| `georag-retrieval` | Search pipeline, ranking |
| `georag-llm` | Embedding generation (Ollama) |
| `georag-store` | Storage adapters (Memory, PostgreSQL) |
| `georag-cli` | Command-line interface |
| `georag-api` | REST API server |

---

## Configuration

### Workspace Config

Located at `.georag/config.toml`:

```toml
crs = 4326
distance_unit = "Meters"
geometry_validity = "Lenient"
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection | (memory) |
| `GEORAG_CRS` | Default CRS | `4326` |
| `GEORAG_EMBEDDER` | Embedder model | `ollama:nomic-embed-text` |

### Global CLI Options

| Option | Description |
|--------|-------------|
| `--json` | JSON output for scripting |
| `--dry-run` | Preview without executing |
| `--explain` | Detailed operation info |
| `--storage` | `memory` or `postgres` |

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Not in a workspace" | Run `georag init` first |
| "Index not built" | Run `georag build` |
| "Embedder unavailable" | Start Ollama: `ollama serve` |
| "CRS mismatch" | Use `--force` or reproject data |
| "Invalid geometry" | Auto-fixed during build (lenient mode) |

### Health Check

```bash
georag doctor --verbose
```

---

## Reference

| Document | Description |
|----------|-------------|
| **[CLI Reference](CLI.md)** | Complete command documentation |
| **[REST API](API.md)** | HTTP API endpoints |
| **[Contributing](CONTRIBUTING.md)** | Development guidelines |

---
