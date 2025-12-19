# CLI Reference

Complete reference for all GeoRAG CLI commands and options.

## Table of Contents

- [Global Flags](#global-flags)
- [Commands](#commands)
  - [init](#init)
  - [add](#add)
  - [build](#build)
  - [query](#query)
  - [inspect](#inspect)
  - [status](#status)
- [Environment Variables](#environment-variables)
- [Exit Codes](#exit-codes)

## Global Flags

These flags can be used with any command:

| Flag            | Description                             | Example                         |
| --------------- | --------------------------------------- | ------------------------------- |
| `--json`        | Output results in JSON format           | `georag status --json`          |
| `--dry-run`     | Show planned actions without executing  | `georag init --dry-run`         |
| `--explain`     | Show detailed explanation of operations | `georag query "text" --explain` |
| `-h, --help`    | Print help information                  | `georag --help`                 |
| `-V, --version` | Print version information               | `georag --version`              |

## Commands

### init

Initialize a new GeoRAG workspace.

**Usage:**

```bash
georag init [PATH] [OPTIONS]
```

**Arguments:**

- `[PATH]` - Workspace directory path (default: current directory)

**Options:**

- `--crs <EPSG>` - CRS EPSG code (default: 4326)
- `--distance-unit <UNIT>` - Distance unit: meters, kilometers, miles, feet (default: meters)
- `--validity-mode <MODE>` - Geometry validity mode: strict, lenient (default: lenient)
- `--force` - Force overwrite if workspace already exists

**Examples:**

```bash
# Initialize in current directory with defaults
georag init

# Initialize with custom CRS
georag init my-workspace --crs 3857

# Initialize with all options
georag init my-workspace \
  --crs 4326 \
  --distance-unit kilometers \
  --validity-mode strict

# Preview initialization
georag init --dry-run

# Get JSON output
georag init --json
```

**Output:**

- Creates `.georag/` directory
- Creates `config.toml` with specified settings
- Creates `datasets/` and `index/` directories
- Creates empty `datasets.json`

---

### add

Add a geospatial dataset to the workspace.

**Usage:**

```bash
georag add <FILE> [OPTIONS]
```

**Arguments:**

- `<FILE>` - Path to the dataset file (GeoJSON, Shapefile, etc.)

**Options:**

- `--name <NAME>` - Dataset name (default: filename)
- `--force` - Override CRS mismatch warning

**Examples:**

```bash
# Add a dataset
georag add data/cities.geojson

# Add with custom name
georag add data/cities.geojson --name "World Cities"

# Force add despite CRS mismatch
georag add data/cities.geojson --force

# Preview addition
georag add data/cities.geojson --dry-run

# Get JSON output
georag add data/cities.geojson --json
```

**Output:**

- Validates geometry
- Extracts CRS metadata
- Displays geometry type, feature count, and CRS
- Warns if CRS differs from workspace CRS
- Copies dataset to `.georag/datasets/`
- Updates `datasets.json`

**Validation:**

- Checks if file exists
- Validates GeoJSON format
- Extracts geometry metadata
- Detects CRS mismatches

---

### build

Build the retrieval index from registered datasets.

**Usage:**

```bash
georag build [OPTIONS]
```

**Options:**

- `--embedder <MODEL>` - Embedder to use (default: ollama:nomic-embed-text)
- `--force` - Force rebuild even if index is up to date

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

# Get JSON output
georag build --json
```

**Process:**

1. Normalizes geometries to workspace CRS
2. Validates and fixes invalid geometries
3. Generates embeddings for text chunks
4. Creates deterministic index hash
5. Saves index state

**Output:**

- Index hash for reproducibility
- Chunk count
- Embedding dimensions
- Embedder information
- Normalization and fix counts

**Requirements:**

- At least one dataset must be registered
- Ollama must be running (for embedding generation)

---

### query

Execute a spatial-semantic query.

**Usage:**

```bash
georag query <QUERY> [OPTIONS]
```

**Arguments:**

- `<QUERY>` - The query text

**Options:**

- `--spatial <PREDICATE>` - Spatial predicate: within, intersects, contains, bbox
- `--geometry <FILE>` - Filter geometry (GeoJSON string or file path)
- `--distance <DISTANCE>` - Distance for proximity queries (e.g., "5km", "100m")
- `--no-rerank` - Disable semantic reranking
- `-k, --top-k <K>` - Number of results to return (default: 10)

**Examples:**

```bash
# Simple query
georag query "What are the main features?"

# Query with spatial filter
georag query "What cities are nearby?" \
  --spatial within \
  --geometry region.geojson

# Query with distance
georag query "What's within 5km?" \
  --spatial within \
  --geometry point.geojson \
  --distance 5km

# Query without reranking
georag query "What features exist?" --no-rerank

# Get detailed explanation
georag query "What's here?" --explain

# Get JSON output
georag query "What's here?" --json
```

**Output:**

- Query plan (spatial predicate, CRS, distance)
- Spatial match count
- Ranked results with sources
- Explanation (with `--explain` flag)

**Requirements:**

- Index must be built
- Ollama must be running (for semantic search)

---

### inspect

Inspect workspace state and metadata.

**Usage:**

```bash
georag inspect <TARGET>
```

**Subcommands:**

- `datasets` - Inspect registered datasets
- `index` - Inspect index metadata
- `crs` - Inspect CRS information
- `config` - Inspect configuration

**Examples:**

#### Inspect Datasets

```bash
georag inspect datasets
georag inspect datasets --json
```

**Output:**

- Dataset ID, name, geometry type
- Feature count
- CRS
- Added timestamp

#### Inspect Index

```bash
georag inspect index
georag inspect index --json
```

**Output:**

- Index hash
- Build timestamp
- Embedder used
- Chunk count
- Embedding dimensions

#### Inspect CRS

```bash
georag inspect crs
georag inspect crs --json
```

**Output:**

- Workspace CRS
- Distance unit
- Per-dataset CRS information
- CRS match indicators

#### Inspect Config

```bash
georag inspect config
georag inspect config --json
```

**Output:**

- Configuration values
- Configuration sources (file, env, CLI, default)
- Configuration precedence information

**Requirements:**

- Must be run within a GeoRAG workspace
- No network access required
- No LLM calls required

---

### status

Show high-level workspace status.

**Usage:**

```bash
georag status [OPTIONS]
```

**Options:**

- `--verbose` - Show detailed status including storage information

**Examples:**

```bash
# Basic status
georag status

# Detailed status
georag status --verbose

# Get JSON output
georag status --json
```

**Output:**

- Workspace location
- Workspace CRS and distance unit
- Dataset count
- Index status (built/not built)
- Index metadata (if built)
- Storage status (with `--verbose`)

**Requirements:**

- Must be run within a GeoRAG workspace
- No network access required
- No heavy computation

---

## Environment Variables

GeoRAG respects the following environment variables:

| Variable               | Description           | Example                                           |
| ---------------------- | --------------------- | ------------------------------------------------- |
| `GEORAG_CRS`           | Default CRS EPSG code | `export GEORAG_CRS=3857`                          |
| `GEORAG_DISTANCE_UNIT` | Default distance unit | `export GEORAG_DISTANCE_UNIT=Kilometers`          |
| `GEORAG_VALIDITY_MODE` | Default validity mode | `export GEORAG_VALIDITY_MODE=Strict`              |
| `GEORAG_EMBEDDER`      | Default embedder      | `export GEORAG_EMBEDDER=ollama:mxbai-embed-large` |

**Precedence:**

```
CLI Arguments > Environment Variables > Config File > Defaults
```

**Example:**

```bash
# Set environment variables
export GEORAG_CRS=3857
export GEORAG_EMBEDDER=ollama:mxbai-embed-large

# These will use the environment variables
georag init
georag build

# CLI arguments override environment variables
georag init --crs 4326
georag build --embedder ollama:nomic-embed-text
```

## Exit Codes

GeoRAG uses standard exit codes:

| Code | Meaning     | Example                            |
| ---- | ----------- | ---------------------------------- |
| `0`  | Success     | Command completed successfully     |
| `1`  | Error       | Command failed (see error message) |
| `2`  | Usage error | Invalid arguments or options       |

**Examples:**

```bash
# Check exit code
georag build
echo $?  # 0 if successful, 1 if failed

# Use in scripts
if georag build; then
  echo "Build successful"
else
  echo "Build failed"
  exit 1
fi
```

## Common Patterns

### Scripting

```bash
#!/bin/bash
set -e  # Exit on error

# Initialize
georag init .

# Add datasets
for file in data/*.geojson; do
  georag add "$file"
done

# Build
georag build

# Query
georag query "What features exist?" > results.txt
```

### JSON Parsing

```bash
# Extract specific fields
georag status --json | jq '.data.crs'
georag status --json | jq '.data.index.built'

# Check build status
if georag status --json | jq -e '.data.index.built == true'; then
  echo "Index is built"
fi
```

### Error Handling

```bash
# Capture output and errors
if ! output=$(georag build --json 2>&1); then
  echo "Build failed: $output"
  exit 1
fi

# Parse error message
error=$(echo "$output" | jq -r '.message')
echo "Error: $error"
```

### Dry-Run Workflow

```bash
# Preview changes
georag build --dry-run

# Review output
read -p "Proceed with build? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  georag build
fi
```

## Tips and Tricks

### 1. Quick Status Check

```bash
# One-liner to check if workspace is ready
georag status --json | jq -r 'if .data.index.built then "Ready" else "Not ready" end'
```

### 2. List All Datasets

```bash
# Get dataset names
georag inspect datasets --json | jq -r '.data.datasets[].name'
```

### 3. Verify Index Hash

```bash
# Get current index hash
current=$(georag inspect index --json | jq -r '.data.hash')

# Rebuild and compare
georag build --force
new=$(georag inspect index --json | jq -r '.data.hash')

if [ "$current" = "$new" ]; then
  echo "Build is deterministic"
fi
```

### 4. Configuration Audit

```bash
# See where each config value comes from
georag inspect config --json | jq '.data | to_entries[] | "\(.key): \(.value.value) (from \(.value.source))"'
```

### 5. Batch Operations

```bash
# Add multiple datasets with error handling
for file in data/*.geojson; do
  if georag add "$file" --json > /dev/null 2>&1; then
    echo "✓ Added: $file"
  else
    echo "✗ Failed: $file"
  fi
done
```

## See Also

- [Output Formatting Guide](output-formatting.md) - JSON output and dry-run mode
- [Configuration Guide](configuration.md) - Configuration management (coming soon)
- [Examples](../examples/) - Example code and workflows
