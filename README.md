<p align="center">
<img src="docs/assets/GeoRAG.png" alt="GeoRAG - Geospatial Data RAG" width="100%">
<br>

<div align="center">

[📖 Quick Start](docs/quick-start.md) <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[📚 Documentation](docs/) <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[🔧 CLI Reference](docs/cli-reference.md) <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
[🤝 Contribute](#contributing)

</div>

<br>

> [!WARNING]
> GeoRAG is in active development! As we build towards 1.0, future updates **will** contain **breaking changes**. We'll annotate changes and provide migration paths as the project evolves.

## What is GeoRAG?

GeoRAG is a Rust library for building **location-aware RAG applications** that combine spatial filtering, lexical search, and vector-based semantic retrieval. Built with privacy and correctness in mind, all processing happens locally without cloud dependencies.

## Features

- **Spatial-Aware Retrieval** - Combine geographic constraints with semantic search
- **Local-First** - All processing happens on your machine, no cloud dependencies
- **Deterministic** - Reproducible index builds with hash verification
- **CRS-Transparent** - Explicit coordinate reference system handling and validation
- **Interactive CLI** - Guided prompts, progress bars, and helpful error messages
- **Flexible Storage** - In-memory or PostgreSQL/PostGIS backends
- **Inspectable** - Every operation can be explained, replayed, and debugged

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/wellywahyudi/georag.git
cd georag
cargo build --release

# Or install CLI
cargo install --path crates/georag-cli
```

### Basic Usage

```bash
# Interactive setup
georag init --interactive

# Add a dataset
georag add cities.geojson

# Build the index
georag build

# Query with spatial constraints
georag query "What are the main features?" \
  --spatial within \
  --geometry bbox.geojson
```

See the [Quick Start Guide](docs/quick-start.md) for detailed walkthrough.

## CLI Commands

### Core Commands

```bash
georag init [PATH]              # Initialize workspace
georag add <FILE>               # Add dataset
georag build                    # Build retrieval index
georag query <TEXT>             # Query with spatial-semantic search
georag status                   # Show workspace status
```

### Database Operations (PostgreSQL)

```bash
georag migrate --database-url <URL>  # Migrate to PostgreSQL
georag db rebuild                    # Rebuild database indexes
georag db stats                      # Show database statistics
georag db vacuum                     # Run maintenance
```

### Utilities

```bash
georag doctor                   # Run health checks
georag status --datasets        # Show datasets only
georag status --config          # Show configuration
```

### Interactive Mode

Add `--interactive` to any command for guided prompts:

```bash
georag init --interactive
georag add --interactive
georag query --interactive
```

See [CLI Reference](docs/cli-reference.md) for complete documentation.

## Configuration

GeoRAG uses layered configuration with clear precedence:

```
CLI Arguments > Environment Variables > Config File > Defaults
```

### Configuration File (`.georag/config.toml`)

```toml
[storage]
backend = "postgres"  # or "memory"

[postgres]
host = "localhost"
port = 5432
database = "georag"
user = "postgres"

[embedder]
default = "ollama:nomic-embed-text"
```

### Environment Variables

```bash
export DATABASE_URL="postgresql://user:pass@localhost/georag"
export GEORAG_CRS=4326
export GEORAG_EMBEDDER=ollama:nomic-embed-text
```

## Examples

### Basic Workflow

```bash
# Initialize workspace
georag init my-project --crs 4326

# Add datasets
cd my-project
georag add data/cities.geojson
georag add data/regions.geojson

# Build index
georag build

# Query
georag query "What cities are in the region?" \
  --spatial within \
  --geometry region-boundary.geojson
```

### Using PostgreSQL

```bash
# Set database URL
export DATABASE_URL="postgresql://localhost/georag"

# Initialize with PostgreSQL
georag init --storage postgres

# Add and build
georag add data.geojson --storage postgres
georag build --storage postgres

# Database maintenance
georag db stats
georag db rebuild
```

### JSON Output for Scripting

```bash
# Get JSON output
georag status --json | jq '.data.index.built'

# Check build status
if georag status --json | jq -e '.data.index.built == true'; then
  echo "Index is built"
fi
```

## Crates

The GeoRAG workspace is organized into focused crates:

- **[georag-core](crates/georag-core)** - Domain models, workspace management, configuration
- **[georag-geo](crates/georag-geo)** - Geometry operations, CRS handling, spatial predicates
- **[georag-retrieval](crates/georag-retrieval)** - Search pipelines, ranking, query execution
- **[georag-llm](crates/georag-llm)** - Embedding generation with local models (Ollama)
- **[georag-store](crates/georag-store)** - Storage abstractions (memory, PostgreSQL)
- **[georag-cli](crates/georag-cli)** - Command-line interface
- **[georag-api](crates/georag-api)** - HTTP API (coming soon)

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo
- GDAL library (libgdal-dev on Ubuntu/Debian, gdal on macOS via Homebrew)
- PROJ library (libproj-dev on Ubuntu/Debian, proj on macOS via Homebrew)
- Ollama (for embedding generation)
- PostgreSQL with PostGIS (optional, for persistent storage)

### Installing GDAL

GDAL is required for reading Shapefile and GeoPackage formats.

**Ubuntu/Debian:**

```bash
sudo apt-get update
sudo apt-get install libgdal-dev libproj-dev
```

**macOS (Homebrew):**

```bash
brew install gdal proj
```

**Arch Linux:**

```bash
sudo pacman -S gdal proj
```

**Verify installation:**

```bash
gdal-config --version
```

If you encounter build issues, ensure `GDAL_HOME` points to your GDAL installation:

```bash
export GDAL_HOME=/usr/local  # or your GDAL installation path
```

**Troubleshooting:**

If you see linking errors during build:

- Ensure GDAL development headers are installed (not just the runtime)
- On macOS, you may need to set: `export GDAL_HOME=$(brew --prefix gdal)`
- On Linux, verify `pkg-config --modversion gdal` returns a version

### Building

```bash
# Build all crates
cargo build --release

# Build specific crate
cargo build -p georag-cli

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy
```

## Design Principles

### Correctness First

- Explicit CRS handling with validation
- Deterministic index builds
- Property-based testing for core operations
- No silent failures or data loss

### Local-First

- All processing happens locally
- No cloud dependencies required
- Optional cloud adapters for scaling
- Full data privacy

### Inspectable

- Every operation can be explained
- Configuration sources are traceable
- Index builds are reproducible
- Dry-run mode for all state changes

## Roadmap

- [x] Core workspace management
- [x] Dataset registration and validation
- [x] Index building with embeddings
- [x] Spatial-semantic query execution
- [x] Interactive CLI with progress indicators
- [x] PostgreSQL/PostGIS storage backend
- [x] Configuration file support
- [x] Health check diagnostics
- [ ] Embedding generation with local models
- [ ] Spatial predicates and operations
- [ ] HTTP API server
- [ ] MCP protocol support
- [ ] Additional embedding models
- [ ] Web UI

## Contributing

We welcome contributions! Here's how you can help:

- 🐛 **Report bugs** - [Open an issue](https://github.com/wellywahyudi/georag/issues/new)
- 💡 **Suggest features** - [Start a discussion](https://github.com/wellywahyudi/georag/discussions)
- 📝 **Improve docs** - Submit documentation improvements
- 🔧 **Submit PRs** - Fix bugs or implement features

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Run `cargo test`, `cargo clippy`, `cargo fmt`
5. Commit your changes
6. Push to the branch
7. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## Documentation

- [Quick Start Guide](docs/quick-start.md) - Get up and running in 5 minutes
- [CLI Reference](docs/cli-reference.md) - Complete command reference
- [Output Formatting](docs/output-formatting.md) - JSON output and dry-run mode

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

GeoRAG is built with excellent open-source tools:

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [geo](https://github.com/georust/geo) - Geospatial primitives and algorithms
- [Ollama](https://ollama.ai/) - Local LLM and embedding models
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [tokio](https://tokio.rs/) - Asynchronous runtime
- [PostgreSQL](https://www.postgresql.org/) & [PostGIS](https://postgis.net/) - Spatial database

---

<div align="center">

**[Get Started →](docs/quick-start.md)**

</div>
