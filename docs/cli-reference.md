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

Add a geospatial dataset or batch process multiple datasets from a directory.

**Usage:**

```bash
georag add <PATH> [OPTIONS]
```

**Arguments:**

- `<PATH>` - Path to a dataset file or directory
  - **File**: Single dataset (GeoJSON, Shapefile, GPX, KML, PDF, DOCX)
  - **Directory**: All supported files in the directory will be processed

**Options:**

- `--name <NAME>` - Dataset name (default: filename, only for single files)
- `--force` - Override CRS mismatch warning
- `-i, --interactive` - Interactive mode with prompts (disabled in batch mode)
- `--track-type <TYPE>` - GPX track type filter: tracks, routes, waypoints, all (GPX only)
- `--folder <PATH>` - KML folder path to extract, e.g., "Parent/Child" (KML only)
- `--geometry <GEOMETRY>` - Associate geometry with documents (PDF, DOCX only)
  - Can be inline GeoJSON: `'{"type":"Point","coordinates":[-122.4,47.6]}'`
  - Or path to GeoJSON file: `location.geojson`
- `--parallel` - Process files in parallel (default: true, batch mode only)

**Supported Formats:**

| Format    | Extensions | Description                    |
| --------- | ---------- | ------------------------------ |
| GeoJSON   | `.geojson` | Standard geospatial JSON       |
| Shapefile | `.shp`     | ESRI Shapefile (requires .dbf) |
| GPX       | `.gpx`     | GPS tracks and waypoints       |
| KML       | `.kml`     | Google Earth format            |
| PDF       | `.pdf`     | Document with text extraction  |
| DOCX      | `.docx`    | Word document                  |

**Examples:**

```bash
# Add a single dataset
georag add data/cities.geojson

# Add with custom name
georag add data/cities.geojson --name "World Cities"

# Force add despite CRS mismatch
georag add data/cities.geojson --force

# Batch process all files in a directory
georag add data/

# Preview batch processing
georag add data/ --dry-run

# Add GPX with track type filter
georag add track.gpx --track-type tracks

# Add KML with folder filter
georag add places.kml --folder "My Places/Favorites"

# Add PDF with associated geometry (inline)
georag add report.pdf --geometry '{"type":"Point","coordinates":[-122.4,47.6]}'

# Add PDF with associated geometry (from file)
georag add report.pdf --geometry location.geojson

# Interactive mode
georag add --interactive
```

**Batch Processing:**

When a directory is provided, GeoRAG will:

1. **Scan** the directory for all supported file formats
2. **Report** the number of files found
3. **Process** each file sequentially with progress updates
4. **Continue** processing even if individual files fail
5. **Display** a summary showing:
   - Total files processed
   - Success/failure counts overall
   - Success/failure counts by format
   - List of successful files with dataset names
   - List of failed files with error messages

**Batch Output Example:**

```
ℹ Scanning directory: data/
ℹ Found 5 supported files
ℹ [1/5] Processing data/cities.geojson (GeoJSON, 1024 bytes)
✓ Added dataset: cities
ℹ [2/5] Processing data/roads.shp (Shapefile, 2048 bytes)
✓ Added dataset: roads
ℹ [3/5] Processing data/track.gpx (GPX, 512 bytes)
✓ Added dataset: track
ℹ [4/5] Processing data/places.kml (KML, 768 bytes)
✓ Added dataset: places
ℹ [5/5] Processing data/report.pdf (PDF, 4096 bytes)
✓ Added dataset: report

Batch Processing Summary
Total Files: 5
Successful: 5
Failed: 0

Summary by Format
GeoJSON: 1 successful, 0 failed
Shapefile: 1 successful, 0 failed
GPX: 1 successful, 0 failed
KML: 1 successful, 0 failed
PDF: 1 successful, 0 failed

Successfully Processed
data/cities.geojson - cities (GeoJSON)
data/roads.shp - roads (Shapefile)
data/track.gpx - track (GPX)
data/places.kml - places (KML)
data/report.pdf - report (PDF)
```

**Error Handling:**

Batch processing is resilient to errors:

```bash
# Even if some files fail, processing continues
georag add data/

# Output shows both successes and failures:
# Batch Processing Summary
# Total Files: 5
# Successful: 3
# Failed: 2
#
# Summary by Format
# GeoJSON: 2 successful, 0 failed
# Shapefile: 0 successful, 1 failed
# PDF: 1 successful, 1 failed
#
# Failed Files
# data/corrupt.shp (Shapefile) - Missing required file: .dbf
# data/empty.pdf (PDF) - PDF contains no extractable text
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

### Batch Processing Workflow

```bash
# 1. Initialize workspace
georag init

# 2. Preview what would be added
georag add data/ --dry-run

# 3. Batch add all files from directory
georag add data/

# 4. Check what was added
georag status --datasets

# 5. Build index
georag build

# 6. Query across all datasets
georag query "search term"
```

### Mixed Format Workflow

```bash
# Add different format types
georag add geospatial/cities.geojson
georag add geospatial/roads.shp
georag add tracks/hike.gpx
georag add places/favorites.kml
georag add documents/report.pdf --geometry region.geojson

# Or batch process mixed formats
georag add mixed_data/

# Build unified index
georag build

# Query across all formats
georag query "what's in this area?" \
  --spatial within \
  --geometry area.geojson
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

# Add datasets individually
for file in data/*.geojson; do
  georag add "$file"
done

# Build
georag build

# Query
georag query "What features exist?" > results.txt
```

### Batch Processing Script

```bash
#!/bin/bash
set -e

# Initialize workspace
georag init my-workspace

# Batch process all supported files
georag add data/

# Check results
if georag status --json | jq -e '.data.dataset_count > 0'; then
  echo "Datasets added successfully"

  # Build index
  georag build

  # Run queries
  georag query "search term" > results.txt
else
  echo "No datasets were added"
  exit 1
fi
```

### Error-Tolerant Batch Script

```bash
#!/bin/bash

# Initialize
georag init .

# Try to add from multiple directories
# Continue even if some directories fail
for dir in data/*; do
  if [ -d "$dir" ]; then
    echo "Processing directory: $dir"
    georag add "$dir" || echo "Warning: Failed to process $dir"
  fi
done

# Build if we have any datasets
dataset_count=$(georag status --json | jq '.data.dataset_count')
if [ "$dataset_count" -gt 0 ]; then
  echo "Building index for $dataset_count datasets"
  georag build
else
  echo "No datasets to index"
fi
```

---

## Tips

1. **Use interactive mode**: `georag init --interactive`
2. **Use doctor for troubleshooting**: `georag doctor`
3. **Use config files for teams**: Share `.georag/config.toml`
4. **Check status before querying**: `georag status`
5. **Use --dry-run to preview**: `georag build --dry-run`
6. **Batch process directories**: `georag add data/` instead of adding files one by one
7. **Preview batch operations**: `georag add data/ --dry-run` to see what would be processed
8. **Mix formats freely**: Batch processing handles GeoJSON, Shapefile, GPX, KML, PDF, and DOCX together
9. **Check batch results**: Review the summary to see which files succeeded or failed
10. **Use format-specific options**: Apply `--track-type` for GPX or `--folder` for KML in batch mode
