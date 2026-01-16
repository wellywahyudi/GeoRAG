# GeoRAG CLI Reference

Complete reference for all GeoRAG command-line interface commands and options.

## Installation

```bash
# Build from source
cargo build --release -p georag-cli

# Install globally
cargo install --path crates/georag-cli
```

## Usage

```bash
georag [OPTIONS] <COMMAND>
```

## Table of Contents

- [Global Options](#global-options)
- [Commands](#commands)
  - [init](#init) - Initialize workspace
  - [add](#add) - Add datasets
  - [build](#build) - Build index
  - [query](#query) - Execute queries
  - [status](#status) - Show status
  - [migrate](#migrate) - Migrate to PostgreSQL
  - [db](#db) - Database management
  - [doctor](#doctor) - Health checks
- [Environment Variables](#environment-variables)
- [Exit Codes](#exit-codes)

---

## Global Options

These options can be used with any command:

| Option | Description | Example |
|--------|-------------|---------|
| `--json` | Output results in JSON format | `georag status --json` |
| `--dry-run` | Show planned actions without executing | `georag init --dry-run` |
| `--explain` | Show detailed explanation of operations | `georag query "text" --explain` |
| `--storage <BACKEND>` | Storage backend: `memory` or `postgres` | `georag --storage postgres build` |
| `-h, --help` | Print help information | `georag --help` |
| `-V, --version` | Print version information | `georag --version` |

---

## Commands

### init

Initialize a new GeoRAG workspace.

```bash
georag init [PATH] [OPTIONS]
```

**Arguments:**

| Argument | Description | Default |
|----------|-------------|---------|
| `PATH` | Workspace directory path | Current directory |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--crs <EPSG>` | CRS EPSG code (e.g., 4326 for WGS 84) | `4326` |
| `--distance-unit <UNIT>` | Distance unit: meters, kilometers, miles, feet | `meters` |
| `--validity-mode <MODE>` | Geometry validity mode: strict, lenient | `lenient` |
| `--force` | Force overwrite if workspace already exists | - |
| `-i, --interactive` | Interactive mode with prompts | - |

**Examples:**

```bash
# Initialize in current directory
georag init

# Interactive setup
georag init --interactive

# Initialize with custom CRS and distance unit
georag init my-workspace --crs 3857 --distance-unit kilometers

# Force overwrite existing workspace
georag init --force
```

---

### add

Add a geospatial dataset or batch process an entire directory.

```bash
georag add <PATH> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `PATH` | Path to dataset file or directory |

**Supported Formats:**

| Format | Extensions | Description |
|--------|------------|-------------|
| GeoJSON | `.geojson`, `.json` | Standard geospatial JSON |
| Shapefile | `.shp` | ESRI Shapefile (requires .dbf, .shx) |
| GPX | `.gpx` | GPS tracks and waypoints |
| KML | `.kml` | Google Earth format |
| PDF | `.pdf` | Documents with text extraction |
| DOCX | `.docx` | Word documents |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--name <NAME>` | Dataset name (single file only) | Filename |
| `--force` | Override CRS mismatch warning | - |
| `-i, --interactive` | Interactive mode with prompts | - |
| `--track-type <TYPE>` | GPX filter: tracks, routes, waypoints, all | - |
| `--folder <PATH>` | KML folder path (e.g., "Parent/Child") | - |
| `--geometry <GEOMETRY>` | Associate geometry with documents | - |
| `--parallel` | Process files in parallel (batch mode) | `true` |
| `-j, --jobs <N>` | Max concurrent jobs (0 = auto) | `0` |
| `--continue-on-error` | Continue if individual files fail | - |

**Examples:**

```bash
# Add a single dataset
georag add data/cities.geojson

# Add with custom name
georag add data/cities.geojson --name "World Cities"

# Batch process directory
georag add data/

# Preview batch processing
georag add data/ --dry-run

# Add GPX with track type filter
georag add trails.gpx --track-type tracks

# Add KML with folder filter
georag add places.kml --folder "My Places/Favorites"

# Add PDF with geometry association (inline)
georag add report.pdf --geometry '{"type":"Point","coordinates":[-122.4,47.6]}'

# Add PDF with geometry from file
georag add report.pdf --geometry location.geojson

# Parallel processing with 8 jobs
georag add data/ --parallel -j 8
```

---

### build

Build the retrieval index from registered datasets.

```bash
georag build [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--embedder <MODEL>` | Embedder model to use | `ollama:nomic-embed-text` |
| `--force` | Force rebuild even if index is up to date | - |

**Examples:**

```bash
# Build with default embedder
georag build

# Build with specific embedder
georag build --embedder ollama:mxbai-embed-large

# Force rebuild
georag build --force

# Preview build
georag build --dry-run
```

**Requirements:**

- At least one dataset must be registered
- Ollama must be running with the specified model

---

### query

Execute a spatial-semantic query.

```bash
georag query <QUERY> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Natural language query text |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--spatial <PREDICATE>` | Spatial predicate: within, intersects, contains, bbox, dwithin | - |
| `--geometry <GEOMETRY>` | Filter geometry (GeoJSON string or file path) | - |
| `--distance <DISTANCE>` | Distance for proximity queries (e.g., "5km", "100m") | - |
| `--must-contain <KEYWORDS>` | Keywords that must appear (comma-separated) | - |
| `--exclude <KEYWORDS>` | Keywords to exclude (comma-separated) | - |
| `--no-rerank` | Disable semantic reranking | - |
| `-k, --top-k <K>` | Number of results to return | `10` |
| `-i, --interactive` | Interactive query builder | - |

**Spatial Predicates:**

| Predicate | Description |
|-----------|-------------|
| `within` | Geometry is completely within filter geometry |
| `intersects` | Geometry overlaps with filter geometry |
| `contains` | Geometry contains the filter geometry |
| `bbox` | Bounding boxes intersect (fast approximation) |
| `dwithin` | Geometry is within specified distance (geodesic) |

**Examples:**

```bash
# Simple query
georag query "What are the main features?"

# Query with spatial filter
georag query "What cities are nearby?" \
  --spatial within \
  --geometry region.geojson

# Distance-based query (DWithin)
georag query "What's within 5km?" \
  --spatial dwithin \
  --geometry point.geojson \
  --distance 5km

# Query with text filtering
georag query "Find restaurants" \
  --must-contain "seafood,outdoor" \
  --exclude "closed,expensive"

# Query without reranking
georag query "What features exist?" --no-rerank

# Get detailed explanation
georag query "What's here?" --explain

# Interactive mode
georag query --interactive
```

---

### status

Show workspace status and information.

```bash
georag status [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--verbose` | Show detailed status |
| `--datasets` | Show only datasets information |
| `--index` | Show only index information |
| `--crs` | Show only CRS information |
| `--config` | Show only configuration |

**Examples:**

```bash
# Show all information
georag status

# Show only datasets
georag status --datasets

# Show only index info
georag status --index

# Get JSON output for scripting
georag status --json | jq '.data.index.built'
```

---

### migrate

Migrate data from in-memory storage to PostgreSQL.

```bash
georag migrate --database-url <URL> [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--database-url <URL>` | PostgreSQL connection string | Required |
| `--batch-size <SIZE>` | Batch size for transfers | `1000` |
| `--verify` | Verify data integrity after migration | - |
| `--dry-run` | Preview migration without executing | - |

**Examples:**

```bash
# Migrate to PostgreSQL
georag migrate --database-url postgresql://user:pass@localhost/georag

# Preview migration
georag migrate --database-url postgresql://localhost/georag --dry-run

# Migrate with verification
georag migrate --database-url postgresql://localhost/georag --verify

# Custom batch size for large datasets
georag migrate --database-url postgresql://localhost/georag --batch-size 500
```

---

### db

Manage database operations (PostgreSQL only).

```bash
georag db <SUBCOMMAND> [OPTIONS]
```

**Subcommands:**

#### db rebuild

Rebuild database indexes for better performance.

```bash
georag db rebuild [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--index <NAME>` | Specific index to rebuild | All indexes |
| `--concurrently` | Non-blocking rebuild | `true` |

```bash
# Rebuild all indexes
georag db rebuild

# Rebuild specific index
georag db rebuild --index idx_features_geom
```

#### db stats

Show database statistics.

```bash
georag db stats [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--index <NAME>` | Specific index to show |

```bash
# Show all statistics
georag db stats

# Get JSON output
georag db stats --json
```

#### db vacuum

Run VACUUM and ANALYZE for database maintenance.

```bash
georag db vacuum [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--table <NAME>` | Specific table to vacuum | All tables |
| `--analyze` | Run ANALYZE after VACUUM | `true` |
| `--full` | Full vacuum (locks table) | - |

```bash
# Vacuum all tables
georag db vacuum

# Full vacuum
georag db vacuum --full
```

---

### doctor

Run health checks and diagnostics.

```bash
georag doctor [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--verbose` | Show detailed diagnostic information |

**Checks Performed:**

- ✓ Workspace detection
- ✓ Configuration validation
- ✓ PostgreSQL connectivity (if configured)
- ✓ Ollama availability
- ✓ Dataset integrity
- ✓ Index status

```bash
# Run health checks
georag doctor

# Detailed diagnostics
georag doctor --verbose
```

---

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@localhost/georag` |
| `GEORAG_CRS` | Default CRS EPSG code | `4326` |
| `GEORAG_DISTANCE_UNIT` | Default distance unit | `Kilometers` |
| `GEORAG_EMBEDDER` | Default embedder model | `ollama:mxbai-embed-large` |

**Configuration Precedence:**

```
CLI Arguments > Environment Variables > Config File > Defaults
```

---

## Exit Codes

| Code | Meaning | Description |
|------|---------|-------------|
| `0` | Success | Command completed successfully |
| `1` | Error | Command failed (see error message) |
| `2` | Usage Error | Invalid arguments or options |

---

## Common Workflows

### Complete Workflow

```bash
# 1. Initialize workspace
georag init --interactive

# 2. Add datasets
georag add cities.geojson
georag add roads.geojson

# 3. Build index
georag build

# 4. Query
georag query "major cities" --top-k 5

# 5. Health check
georag doctor
```

### Batch Processing

```bash
# Preview what would be added
georag add data/ --dry-run

# Batch add all files
georag add data/

# Check status
georag status --datasets

# Build unified index
georag build
```

### PostgreSQL Workflow

```bash
# Set database URL
export DATABASE_URL="postgresql://user:pass@localhost/georag"

# Initialize with PostgreSQL
georag --storage postgres init

# Add datasets
georag --storage postgres add data.geojson

# Build index
georag --storage postgres build

# Database maintenance
georag db stats
georag db vacuum
```

### JSON Output for Scripting

```bash
# Check if index is built
if georag status --json | jq -e '.data.index.built == true'; then
  echo "Index is ready"
fi

# Get dataset count
georag status --json | jq '.data.dataset_count'

# Query and process results
georag query "search" --json | jq '.sources[].excerpt'
```

---

## Tips

1. **Use interactive mode** for guided setup: `georag init --interactive`
2. **Preview with dry-run** before destructive operations: `georag build --dry-run`
3. **Use doctor** for troubleshooting: `georag doctor --verbose`
4. **Batch process directories** instead of individual files: `georag add data/`
5. **Use text filtering** to narrow results: `--must-contain "keyword"`
6. **Use DWithin** for geodesic distance queries: `--spatial dwithin --distance 5km`
7. **Get JSON output** for automation: `georag status --json`
8. **Share configuration** via `.georag/config.toml`
9. **Check status** before querying: `georag status`
10. **Use parallel processing** for large batch imports: `georag add data/ -j 8`

---

## See Also

- [Documentation](README.md)
- [REST API Reference](API.md)
