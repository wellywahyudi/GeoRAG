<p align="center">
<img src="docs/assets/GeoRAG.png" alt="GeoRAG - Geospatial Data RAG" width="100%">
</p>

<div align="center">

[📖 Quick Start](#quick-start) &nbsp;•&nbsp;
[📚 Documentation](docs/) &nbsp;•&nbsp;
[🔧 CLI Reference](docs/CLI.md) &nbsp;•&nbsp;
[🌐 API Reference](docs/API.md)

</div>

> [!WARNING]
> GeoRAG is in active development. Future updates may contain breaking changes.

## What is GeoRAG?

GeoRAG is a Rust library for building **location-aware RAG applications** that combine spatial filtering with semantic search. All processing happens locally without cloud dependencies.

## Features

| Category | Capabilities |
|----------|-------------|
| **Formats** | GeoJSON, Shapefile, GPX, KML, PDF, DOCX |
| **Spatial** | Within, Intersects, Contains, BBox, DWithin |
| **Text** | Must-contain, exclude keyword filtering |
| **Storage** | In-memory, PostgreSQL + PostGIS |
| **Embeddings** | Ollama (local models) |
| **API** | CLI + REST API |

## Quick Start

```bash
# Install
git clone https://github.com/wellywahyudi/georag.git
cd georag && cargo install --path crates/georag-cli

# Start Ollama
ollama pull nomic-embed-text

# Initialize, add data, build, query
georag init my-project && cd my-project
georag add cities.geojson
georag build
georag query "What cities are nearby?" --spatial dwithin --distance 5km
```

## CLI Commands

```bash
georag init [PATH]     # Initialize workspace
georag add <FILE>      # Add dataset (file or directory)
georag build           # Build retrieval index
georag query <TEXT>    # Spatial-semantic search
georag status          # Show workspace status
georag doctor          # Run diagnostics
```

**PostgreSQL:**
```bash
georag migrate --database-url <URL>   # Migrate to PostgreSQL
georag db rebuild                     # Rebuild indexes
georag db vacuum                      # Maintenance
```

**Options:** `--json` (scripting), `--dry-run` (preview), `--explain` (details)

## REST API

```bash
# Start server
DATABASE_URL=postgresql://localhost/georag georag-api
```

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `POST /api/v1/query` | Execute query |
| `GET /api/v1/datasets` | List datasets |
| `POST /api/v1/ingest` | Upload dataset |
| `GET /api/v1/index/integrity` | Index state |

## Configuration

```bash
# Environment
export DATABASE_URL="postgresql://user:pass@localhost/georag"
export GEORAG_EMBEDDER="ollama:nomic-embed-text"
```

```toml
# .georag/config.toml
crs = 4326
distance_unit = "Meters"
```

## Development

```bash
cargo build --release
cargo test --all
cargo clippy --all-targets -- -D warnings
```

## Contributing

1. Fork → 2. Branch → 3. Code → 4. Test → 5. PR

See [CONTRIBUTING.md](docs/CONTRIBUTING.md)

## License

MIT - See [LICENSE](LICENSE)

## Acknowledgments

Built with [Rust](https://www.rust-lang.org/), [GeoRust](https://georust.org/), [Ollama](https://ollama.ai/), [PostgreSQL](https://www.postgresql.org/) + [PostGIS](https://postgis.net/)
