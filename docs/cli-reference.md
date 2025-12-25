# CLI Reference

Complete reference for all GeoRAG CLI commands and options.

## Table of Contents

- [Global Options](#global-options)
- [Commands](#commands)
  - [init](#init)
  - [add](#add)
  - [build](#build)
  - [query](#query)
  - [status](#status)
  - [migrate](#migrate)
  - [db](#db)
  - [doctor](#doctor)
- [Configuration](#configuration)
- [Exit Codes](#exit-codes)

## Global Options

These options can be used with any command:

| Option                | Description                             | Example                           |
| --------------------- | --------------------------------------- | --------------------------------- |
| `--json`              | Output results in JSON format           | `georag status --json`            |
| `--dry-run`           | Show planned actions without executing  | `georag init --dry-run`           |
| `--explain`           | Show detailed explanation of operations | `georag query "text" --explain`   |
| `--storage <BACKEND>` | Storage backend: `memory` or `postgres` | `georag --storage postgres build` |
| `-h, --help`          | Print help information                  | `georag --help`                   |
| `-V, --version`       | Print version information               | `georag --version`                |

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
- `-i, --interactive` - Interactive mode with prompts

**Examples:**

```bash
# Initialize in current directory
georag init

# Interactive setup
georag init --interactive

# Initialize with custom CRS
georag init my-workspace --crs 3857

# Initialize with all options
georag init my-workspace \
  --crs 4326 \
  --distance-unit kilometers \
  --validity-mode strict
```

---

### add

Add a geospatial dataset to the workspace.

**Usage:**

```bash
georag add <PATH> [OPTIONS]
```

**Arguments:**

- `<PATH>` - Path to the dataset file (GeoJSON, Shapefile, etc.)

**Options:**

- `--name <NAME>` - Dataset name (default: filename)
- `--force` - Override CRS mismatch warning
- `-i, --interactive` - Interactive mode with prompts

**Examples:**

```bash
# Add a dataset
georag add data/cities.geojson

# Interactive mode
georag add --interactive

# Add with custom name
georag add data/cities.geojson --name "World Cities"

# Force add despite CRS mismatch
georag add data/cities.geojson --force
```

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
```

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
- `-i, --interactive` - Interactive query builder

**Examples:**

```bash
# Simple query
georag query "What are the main features?"

# Interactive query builder
georag query --interactive

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
```

---

### status

Show workspace status and information.

**Usage:**

```bash
georag status [OPTIONS]
```

**Options:**

- `--verbose` - Show detailed status
- `--datasets` - Show only datasets information
- `--index` - Show only index information
- `--crs` - Show only CRS information
- `--config` - Show only configuration

**Examples:**

```bash
# Show all information
georag status

# Show only datasets
georag status --datasets

# Show only index info
georag status --index

# Show only CRS info
georag status --crs

# Show only configuration
georag status --config

# Detailed status
georag status --verbose
```

**Output:**

- Workspace location and CRS
- Dataset count and details
- Index status and metadata
- Configuration values and sources
- Storage status (with `--verbose`)

---

### migrate

Migrate data from in-memory storage to PostgreSQL.

**Usage:**

```bash
georag migrate --database-url <URL> [OPTIONS]
```

**Options:**

- `--database-url <URL>` - PostgreSQL database URL (required)
- `--batch-size <SIZE>` - Batch size for transferring records (default: 1000)
- `--verify` - Verify data integrity after migration

**Examples:**

```bash
# Migrate to PostgreSQL
georag migrate --database-url postgresql://user:pass@localhost/georag

# Preview migration
georag migrate --database-url postgresql://localhost/georag --dry-run

# Migrate with verification
georag migrate --database-url postgresql://localhost/georag --verify

# Custom batch size
georag migrate --database-url postgresql://localhost/georag --batch-size 500
```

**Process:**

1. Connects to PostgreSQL
2. Runs migrations
3. Transfers datasets
4. Transfers features
5. Transfers embeddings
6. Optionally verifies integrity

---

### db

Manage database operations (PostgreSQL only).

**Usage:**

```bash
georag db <SUBCOMMAND> [OPTIONS]
```

**Subcommands:**

- `rebuild` - Rebuild database indexes
- `stats` - Show database statistics
- `vacuum` - Run VACUUM and ANALYZE for maintenance

#### db rebuild

Rebuild database indexes for better performance.

**Usage:**

```bash
georag db rebuild [OPTIONS]
```

**Options:**

- `--index <NAME>` - Specific index to rebuild (rebuilds all if not specified)
- `--concurrently` - Rebuild indexes concurrently (non-blocking, default: true)

**Examples:**

```bash
# Rebuild all indexes
georag db rebuild

# Rebuild specific index
georag db rebuild --index idx_features_geom

# Preview rebuild
georag db rebuild --dry-run
```

#### db stats

Show database statistics.

**Usage:**

```bash
georag db stats [OPTIONS]
```

**Options:**

- `--index <NAME>` - Specific index to show stats for (shows all if not specified)

**Examples:**

```bash
# Show all index statistics
georag db stats

# Show specific index stats
georag db stats --index idx_features_geom

# Get JSON output
georag db stats --json
```

**Output:**

- Index name and table
- Index type
- Size in bytes
- Row count
- Last vacuum/analyze timestamps

#### db vacuum

Run VACUUM and ANALYZE for database maintenance.

**Usage:**

```bash
georag db vacuum [OPTIONS]
```

**Options:**

- `--table <NAME>` - Specific table to vacuum (vacuums all if not specified)
- `--analyze` - Run ANALYZE after VACUUM (default: true)
- `--full` - Run FULL vacuum (locks table, reclaims more space)

**Examples:**

```bash
# Vacuum all tables
georag db vacuum

# Vacuum specific table
georag db vacuum --table features

# Full vacuum
georag db vacuum --full

# Preview vacuum
georag db vacuum --dry-run
```

---

### doctor

Run health checks and diagnostics.

**Usage:**

```bash
georag doctor [OPTIONS]
```

**Options:**

- `--verbose` - Show detailed diagnostic information

**Examples:**

```bash
# Run health checks
georag doctor

# Detailed diagnostics
georag doctor --verbose
```

**Checks:**

- ✓ Workspace detection
- ✓ Configuration validation
- ✓ PostgreSQL connectivity (if configured)
- ✓ Ollama availability
- ✓ Dataset integrity
- ✓ Index status

**Output:**

- Pass/fail status for each check
- Suggestions for fixing issues
- Overall health score

---

## Configuration

### Configuration File

GeoRAG supports `.georag/config.toml` for persistent configuration:

```toml
[storage]
backend = "postgres"  # or "memory"

[postgres]
host = "localhost"
port = 5432
database = "georag"
user = "postgres"
# password can be in env var or .pgpass

[postgres.pool]
min_connections = 2
max_connections = 10
acquire_timeout = 30
idle_timeout = 600

[embedder]
default = "ollama:nomic-embed-text"
```

### Environment Variables

| Variable               | Description                  | Example                                           |
| ---------------------- | ---------------------------- | ------------------------------------------------- |
| `DATABASE_URL`         | PostgreSQL connection string | `postgresql://user:pass@localhost/georag`         |
| `GEORAG_CRS`           | Default CRS EPSG code        | `export GEORAG_CRS=3857`                          |
| `GEORAG_DISTANCE_UNIT` | Default distance unit        | `export GEORAG_DISTANCE_UNIT=Kilometers`          |
| `GEORAG_EMBEDDER`      | Default embedder             | `export GEORAG_EMBEDDER=ollama:mxbai-embed-large` |

### Configuration Precedence

```
CLI Arguments > Environment Variables > Config File > Defaults
```

---

## Exit Codes

| Code | Meaning     | Example                            |
| ---- | ----------- | ---------------------------------- |
| `0`  | Success     | Command completed successfully     |
| `1`  | Error       | Command failed (see error message) |
| `2`  | Usage error | Invalid arguments or options       |

---

## Common Patterns

### Complete Workflow

```bash
# 1. Initialize workspace
georag init --interactive

# 2. Add datasets
georag add cities.geojson
georag add roads.geojson

# 3. Check status
georag status

# 4. Build index
georag build

# 5. Query
georag query "major cities" --top-k 10

# 6. Health check
georag doctor
```

### Using PostgreSQL

```bash
# Set database URL
export DATABASE_URL="postgresql://user:pass@localhost/georag"

# Or use config file (.georag/config.toml)
# [postgres]
# host = "localhost"
# database = "georag"

# Initialize with PostgreSQL
georag init --storage postgres

# Add and build
georag add data.geojson --storage postgres
georag build --storage postgres

# Query
georag query "search" --storage postgres

# Database maintenance
georag db stats
georag db rebuild
georag db vacuum
```

### JSON Output

```bash
# Get JSON output
georag status --json | jq '.data.crs'
georag status --json | jq '.data.index.built'

# Check build status
if georag status --json | jq -e '.data.index.built == true'; then
  echo "Index is built"
fi
```

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

---

## Tips

1. **Use interactive mode**: `georag init --interactive`
2. **Use doctor for troubleshooting**: `georag doctor`
3. **Use config files for teams**: Share `.georag/config.toml`
4. **Check status before querying**: `georag status`
5. **Use --dry-run to preview**: `georag build --dry-run`
