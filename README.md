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
> GeoRAG is in active development! As we build towards 1.0, future updates **will** contain **breaking changes**. We'll annotate changes and provide migration paths as the project evolves. Join us on this journey to build the best geospatial RAG system in Rust!

## Table of Contents

- [What is GeoRAG?](#what-is-georag)
- [Features](#features)
- [Who's Using GeoRAG?](#whos-using-georag)
- [Get Started](#get-started)
  - [Installation](#installation)
  - [Simple Example](#simple-example)
- [Architecture](#architecture)
- [Integrations](#integrations)
- [Documentation](#documentation)
- [Contributing](#contributing)

## What is GeoRAG?

GeoRAG is a Rust library for building **location-aware RAG applications** that combine spatial filtering, lexical search, and vector-based semantic retrieval. Built with privacy and correctness in mind, all processing happens locally without cloud dependencies.

More information can be found in the [Quick Start Guide](docs/quick-start.md) and [API Documentation](https://docs.rs/georag-core/latest/georag/).

## Features

- **Spatial-Aware Retrieval** - Combine geographic constraints with semantic search
- **Local-First** - All processing happens on your machine, no cloud dependencies
- **Deterministic** - Reproducible index builds with hash verification
- **CRS-Transparent** - Explicit coordinate reference system handling and validation
- **Flexible Output** - Human-readable terminal output or machine-readable JSON
- **Inspectable** - Every operation can be explained, replayed, and debugged
- **Hexagonal Architecture** - Clean separation between core logic and adapters
- **Property-Based Testing** - Correctness properties verified across all inputs

## Get Started

### Installation

```bash
cargo add georag-core
```

Or install the CLI:

```bash
# Clone and build
git clone https://github.com/wellywahyudi/georag.git
cd georag
cargo install --path crates/georag-cli
```

**CLI Usage:**

```bash
# Initialize a workspace
georag init my-workspace --crs 4326

# Add a geospatial dataset
cd my-workspace
georag add data/cities.geojson

# Build the retrieval index
georag build

# Query with spatial constraints
georag query "What are the main features?" \
  --spatial within \
  --geometry bbox.geojson \
  --distance 5km
```

Note: Using `#[tokio::main]` requires enabling tokio's `macros` and `rt-multi-thread` features (`cargo add tokio --features macros,rt-multi-thread`).

You can find more examples in each crate's `examples` directory (e.g., [`georag-core/examples`](./crates/georag-core/examples)). Detailed walkthroughs are available in our [documentation](docs/).

## Architecture

GeoRAG follows hexagonal architecture principles with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                    Adapters Layer                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │   CLI    │  │   API    │  │   MCP    │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└─────────────────────────────────────────────────────────┘
                         │
┌─────────────────────────────────────────────────────-───┐
│                  Application Layer                      │
│  ┌──────────────────────────────────────────────────┐   │
│  │           Use Cases & Orchestration              │   │
│  └──────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────-┘
                         │
┌─────────────────────────────────────────────────────────┐
│                    Domain Core                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ georag-  │  │ georag-  │  │ georag-  │               │
│  │  core    │  │   geo    │  │retrieval │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└─────────────────────────────────────────────────────────┘
                         │
┌─────────────────────────────────────────────────────────┐
│              Infrastructure Adapters                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ georag-  │  │ georag-  │  │ georag-  │               │
│  │  store   │  │   llm    │  │   api    │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└─────────────────────────────────────────────────────────┘
```

### Crates

The GeoRAG workspace is organized into focused crates:

- **[georag-core](crates/georag-core)** - Domain models, workspace management, configuration
- **[georag-geo](crates/georag-geo)** - Geometry operations, CRS handling, spatial predicates
- **[georag-retrieval](crates/georag-retrieval)** - Search pipelines, ranking, query execution
- **[georag-llm](crates/georag-llm)** - Embedding generation with local models (Ollama)
- **[georag-store](crates/georag-store)** - Storage abstractions (in-memory, file-based)
- **[georag-cli](crates/georag-cli)** - Command-line interface
- **[georag-api](crates/georag-api)** - HTTP API (coming soon)

## Integrations

### Vector Stores

Vector stores are available as separate companion crates (coming soon):

- **PostgreSQL/PostGIS**: `georag-postgres` (planned)
- **SQLite**: `georag-sqlite` (planned)
- **In-Memory**: Built into `georag-store`

### Embedding Providers

- **Ollama**: Built into `georag-llm` (local embeddings)
- **OpenAI**: `georag-openai` (planned)
- **Anthropic**: `georag-anthropic` (planned)

### Storage Backends

- **File System**: Built into `georag-store`
- **S3**: `georag-s3` (planned)
- **Cloud Storage**: Additional providers planned

## CLI Commands

### Workspace Management

```bash
# Initialize a new workspace
georag init [PATH] --crs <EPSG> --distance-unit <UNIT>

# Show workspace status
georag status [--verbose]
```

### Dataset Operations

```bash
# Add a geospatial dataset
georag add <FILE> [--name <NAME>] [--force]

# List registered datasets
georag inspect datasets
```

### Index Building

```bash
# Build the retrieval index
georag build [--embedder <MODEL>] [--force]

# Inspect index metadata
georag inspect index
```

### Querying

```bash
# Query with spatial filter
georag query "your question" \
  --spatial <PREDICATE> \
  --geometry <FILE> \
  --distance <DISTANCE> \
  [--no-rerank] \
  [--explain]

# Spatial predicates: within, intersects, contains, bbox
```

### Inspection

```bash
# Inspect datasets
georag inspect datasets

# Inspect index
georag inspect index

# Inspect CRS information
georag inspect crs

# Inspect configuration
georag inspect config
```

## Output Formatting

GeoRAG supports flexible output formatting for different use cases:

### JSON Output

Perfect for scripting and automation:

```bash
# Get JSON output
georag init --json
georag status --json
georag build --json

# Parse with jq
georag status --json | jq '.data.index.built'
```

### Dry-Run Mode

Preview operations without making changes:

```bash
# See what would happen
georag init --dry-run
georag add dataset.geojson --dry-run
georag build --dry-run

# Combine with JSON
georag build --dry-run --json
```

See the [Output Formatting Guide](docs/output-formatting.md) for detailed documentation.

## Configuration

GeoRAG uses layered configuration with clear precedence:

```
CLI Arguments > Environment Variables > Config File > Defaults
```

### Configuration File

Located at `.georag/config.toml`:

```toml
# Coordinate Reference System (EPSG code)
crs = 4326

# Distance unit for spatial operations
distance_unit = "Meters"

# Geometry validity mode
geometry_validity = "Lenient"
```

### Environment Variables

```bash
export GEORAG_CRS=4326
export GEORAG_DISTANCE_UNIT=Kilometers
export GEORAG_EMBEDDER=ollama:nomic-embed-text
```

### CLI Arguments

```bash
georag build --embedder ollama:mxbai-embed-large
```

Inspect configuration sources:

```bash
georag inspect config
```

## Examples

### Example 1: Basic Workflow

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

### Example 2: CI/CD Integration

```bash
#!/bin/bash
set -e

# Initialize with JSON output
georag init . --json > init.json

# Add datasets
for file in data/*.geojson; do
  georag add "$file" --json >> datasets.json
done

# Build and verify
georag build --json > build.json

# Check build status
if jq -e '.status == "success"' build.json; then
  echo "Build successful"
else
  echo "Build failed"
  exit 1
fi
```

### Example 3: Safe Operations

```bash
# Preview changes
georag build --dry-run

# Review output, then execute
georag build

# Verify with status
georag status
```

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo
- Ollama (for embedding generation)

### Building

```bash
# Build all crates
cargo build

# Build with release optimizations
cargo build --release

# Build specific crate
cargo build -p georag-cli
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p georag-core

# Run integration tests
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check without building
cargo check
```

## Project Structure

```
georag/
├── crates/
│   ├── georag-core/       # Core domain logic
│   ├── georag-geo/        # Geometry operations
│   ├── georag-retrieval/  # Search and ranking
│   ├── georag-llm/        # Embedding generation
│   ├── georag-store/      # Storage abstractions
│   ├── georag-cli/        # Command-line interface
│   └── georag-api/        # HTTP API (future)
├── docs/                  # Documentation
├── examples/              # Example code
├── Cargo.toml            # Workspace manifest
└── README.md             # This file
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

### Extensible

- Port/adapter pattern for integrations
- Clean separation of concerns
- Easy to add new storage backends
- Easy to add new embedding models

## Roadmap

- [x] Core workspace management
- [x] Dataset registration and validation
- [x] Index building with embeddings
- [x] Spatial-semantic query execution
- [x] JSON output and dry-run mode
- [ ] Property-based testing suite
- [ ] HTTP API server
- [ ] MCP protocol support
- [ ] Additional storage backends (PostgreSQL/PostGIS)
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
3. Make your changes
4. Add tests (`cargo test`)
5. Run lints (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines (coming soon).

## Documentation

### Getting Started

- [Quick Start Guide](docs/quick-start.md) - Get up and running in 5 minutes
- [CLI Reference](docs/cli-reference.md) - Complete command reference
- [Output Formatting Guide](docs/output-formatting.md) - JSON output and dry-run mode

### Advanced Topics

- [Configuration Guide](docs/configuration.md) - Configuration management (coming soon)
- [Architecture Guide](docs/architecture.md) - System architecture (coming soon)
- [API Documentation](docs/api.md) - HTTP API reference (coming soon)
- [Contributing Guide](CONTRIBUTING.md) - How to contribute (coming soon)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Geometry operations powered by [geo](https://github.com/georust/geo)
- Embeddings via [Ollama](https://ollama.ai/)
- CLI framework: [clap](https://github.com/clap-rs/clap)

## Community & Support

- 📖 **[Documentation](docs/)** - Comprehensive guides and references
- 🐛 **[Issue Tracker](https://github.com/wellywahyudi/georag/issues)** - Report bugs and request features
- 💬 **[Discussions](https://github.com/wellywahyudi/georag/discussions)** - Ask questions and share ideas
- 📧 **Email** - Contact the maintainers

## Acknowledgments

GeoRAG is built with excellent open-source tools:

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [geo](https://github.com/georust/geo) - Geospatial primitives and algorithms
- [Ollama](https://ollama.ai/) - Local LLM and embedding models
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [tokio](https://tokio.rs/) - Asynchronous runtime
