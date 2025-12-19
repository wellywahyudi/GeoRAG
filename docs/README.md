# GeoRAG Documentation

Welcome to the GeoRAG documentation! This guide will help you get started with GeoRAG and make the most of its features.

## Getting Started

New to GeoRAG? Start here:

1. **[Quick Start Guide](quick-start.md)** - Get up and running in 5 minutes

   - Installation
   - Your first workspace
   - Basic workflow
   - Common patterns

2. **[CLI Reference](cli-reference.md)** - Complete command documentation

   - All commands and options
   - Environment variables
   - Exit codes
   - Tips and tricks

3. **[Output Formatting Guide](output-formatting.md)** - JSON output and dry-run mode
   - JSON output for automation
   - Dry-run mode for safety
   - Combining modes
   - Best practices

## Core Concepts

### Workspace

A GeoRAG workspace is a directory containing:

- `.georag/` - Configuration and data directory
- `config.toml` - Workspace configuration
- `datasets.json` - Registered datasets
- `datasets/` - Dataset files
- `index/` - Built retrieval index

### Coordinate Reference Systems (CRS)

GeoRAG is CRS-transparent, meaning:

- Every workspace has a defined CRS
- Every dataset has a detected CRS
- CRS mismatches are explicitly warned
- Geometries are normalized during build

### Index Building

The index building process:

1. Normalizes geometries to workspace CRS
2. Validates and fixes invalid geometries
3. Generates embeddings for text chunks
4. Creates a deterministic index hash
5. Saves index state for reproducibility

### Retrieval Pipeline

Query execution follows this pipeline:

1. **Spatial Filtering** - Filter by geographic constraints
2. **Semantic Ranking** - Rank by embedding similarity
3. **Result Grounding** - Link results to source features
4. **Explanation** - Provide reasoning (optional)

## Architecture

GeoRAG follows hexagonal architecture:

```
┌─────────────────────────────────────────┐
│         Adapters (CLI, API)             │
└─────────────────────────────────────────┘
                  │
┌─────────────────────────────────────────┐
│        Application Layer                │
│     (Use Cases, Orchestration)          │
└─────────────────────────────────────────┘
                  │
┌─────────────────────────────────────────┐
│          Domain Core                    │
│  (georag-core, georag-geo, retrieval)   │
└─────────────────────────────────────────┘
                  │
┌─────────────────────────────────────────┐
│    Infrastructure Adapters              │
│   (georag-store, georag-llm)            │
└─────────────────────────────────────────┘
```

### Crates

- **georag-core** - Domain models, workspace, configuration
- **georag-geo** - Geometry operations, CRS, spatial predicates
- **georag-retrieval** - Search pipelines, ranking
- **georag-llm** - Embedding generation (Ollama)
- **georag-store** - Storage abstractions
- **georag-cli** - Command-line interface
- **georag-api** - HTTP API (future)

## Features

### ✅ Implemented

- [x] Workspace initialization and management
- [x] Dataset registration with validation
- [x] CRS detection and mismatch warnings
- [x] Index building with embeddings
- [x] Spatial-semantic query execution
- [x] JSON output mode
- [x] Dry-run mode
- [x] Configuration inspection
- [x] Deterministic index builds

### 🚧 In Progress

- [ ] Property-based testing suite
- [ ] HTTP API server
- [ ] MCP protocol support

### 📋 Planned

- [ ] Additional storage backends (PostgreSQL/PostGIS)
- [ ] Additional embedding models
- [ ] Web UI
- [ ] Advanced spatial operations
- [ ] Query optimization

## Use Cases

### 1. Geospatial Data Exploration

Explore and query geospatial datasets with natural language:

```bash
georag init exploration
georag add cities.geojson
georag build
georag query "What are the largest cities?"
```

### 2. Location-Based Search

Find information within geographic constraints:

```bash
georag query "What restaurants are nearby?" \
  --spatial within \
  --geometry location.geojson \
  --distance 1km
```

### 3. Multi-Dataset Analysis

Query across multiple geospatial datasets:

```bash
georag add cities.geojson
georag add roads.geojson
georag add regions.geojson
georag build
georag query "What infrastructure exists in urban areas?"
```

### 4. CI/CD Integration

Automate geospatial data processing:

```bash
georag init --json
georag add data/*.geojson --json
georag build --json
georag query "Generate summary" --json
```

### 5. Data Quality Validation

Validate and fix geospatial data:

```bash
georag add dataset.geojson
georag build  # Reports invalid geometries and fixes
georag inspect datasets  # View metadata
```

## Best Practices

### 1. Start with Dry-Run

Always preview operations first:

```bash
georag build --dry-run
georag add dataset.geojson --dry-run
```

### 2. Use JSON for Automation

Leverage JSON output in scripts:

```bash
georag status --json | jq '.data.index.built'
```

### 3. Check Status Regularly

Monitor workspace state:

```bash
georag status
georag inspect datasets
georag inspect index
```

### 4. Validate CRS

Ensure CRS consistency:

```bash
georag inspect crs
```

### 5. Keep Index Updated

Rebuild after changes:

```bash
georag add new-dataset.geojson
georag build --force
```

## Configuration

### Layered Configuration

GeoRAG uses layered configuration with clear precedence:

```
CLI Arguments > Environment Variables > Config File > Defaults
```

### Configuration File

Located at `.georag/config.toml`:

```toml
crs = 4326
distance_unit = "Meters"
geometry_validity = "Lenient"
```

### Environment Variables

```bash
export GEORAG_CRS=4326
export GEORAG_DISTANCE_UNIT=Kilometers
export GEORAG_EMBEDDER=ollama:nomic-embed-text
```

### Inspection

View configuration sources:

```bash
georag inspect config
```

## Troubleshooting

### Common Issues

1. **"Not in a GeoRAG workspace"**

   - Solution: Run `georag init` or navigate to workspace directory

2. **"Index not built"**

   - Solution: Run `georag build`

3. **"Embedder unavailable"**

   - Solution: Ensure Ollama is running and model is pulled

4. **"CRS mismatch"**

   - Solution: Use `--force` flag or reproject data

5. **"Invalid geometry"**
   - Solution: GeoRAG will attempt to fix during build

### Getting Help

- 📖 Read the documentation
- 🐛 [Report issues](https://github.com/wellywahyudi/georag/issues)
- 💬 [Join discussions](https://github.com/wellywahyudi/georag/discussions)
- 📧 Contact maintainers

## Examples

See the [examples directory](../examples/) for:

- Basic workflows
- Advanced queries
- Scripting examples
- CI/CD integration
- Custom configurations

## Contributing

We welcome contributions! See [CONTRIBUTING.md](../CONTRIBUTING.md) for:

- Development setup
- Code style guidelines
- Testing requirements
- Pull request process

## API Reference

### CLI Commands

- `init` - Initialize workspace
- `add` - Add dataset
- `build` - Build index
- `query` - Execute query
- `inspect` - Inspect state
- `status` - Show status

See [CLI Reference](cli-reference.md) for complete documentation.

### Rust API

For programmatic access, see the crate documentation:

```bash
cargo doc --open
```

## Design Principles

### Correctness First

- Explicit CRS handling
- Deterministic operations
- Property-based testing
- No silent failures

### Local-First

- All processing local
- No cloud dependencies
- Optional cloud adapters
- Full data privacy

### Inspectable

- Every operation explainable
- Configuration traceable
- Index reproducible
- Dry-run for all changes

### Extensible

- Port/adapter pattern
- Clean separation
- Easy to extend
- Pluggable components

## Roadmap

See the [main README](../README.md#roadmap) for the project roadmap.

## License

GeoRAG is licensed under the MIT License. See [LICENSE](../LICENSE) for details.

## Acknowledgments

Built with:

- [Rust](https://www.rust-lang.org/)
- [geo](https://github.com/georust/geo)
- [Ollama](https://ollama.ai/)
- [clap](https://github.com/clap-rs/clap)

---

**Questions?** Open an [issue](https://github.com/wellywahyudi/georag/issues) or start a [discussion](https://github.com/wellywahyudi/georag/discussions).
