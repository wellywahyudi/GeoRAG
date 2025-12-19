# Quick Start Guide

Get up and running with GeoRAG in 5 minutes.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.70+**: [Install Rust](https://www.rust-lang.org/tools/install)
- **Ollama**: [Install Ollama](https://ollama.ai/) for embedding generation
- **Git**: For cloning the repository

## Installation

### 1. Clone and Build

```bash
# Clone the repository
git clone https://github.com/wellywahyudi/georag.git
cd georag

# Build the project
cargo build --release

# Install the CLI
cargo install --path crates/georag-cli
```

### 2. Verify Installation

```bash
# Check version
georag --version

# View help
georag --help
```

### 3. Start Ollama

```bash
# Pull the embedding model
ollama pull nomic-embed-text

# Verify Ollama is running
ollama list
```

## Your First Workspace

### Step 1: Initialize

Create a new workspace with default settings:

```bash
# Create a project directory
mkdir my-georag-project
cd my-georag-project

# Initialize workspace
georag init
```

**Output:**

```
✓ Initialized GeoRAG workspace at /path/to/my-georag-project

Configuration
CRS: EPSG:4326
Distance Unit: Meters
Validity Mode: Lenient
```

**What happened:**

- Created `.georag/` directory
- Created `config.toml` with default settings
- Created `datasets/` and `index/` directories
- Ready to add datasets

### Step 2: Add a Dataset

Add a GeoJSON file to your workspace:

```bash
# Download sample data (or use your own)
curl -o cities.geojson https://example.com/cities.geojson

# Add to workspace
georag add cities.geojson
```

**Output:**

```
✓ Added dataset: cities

Dataset Information
Geometry Type: Point
Feature Count: 150
CRS: EPSG:4326
```

**What happened:**

- Validated the GeoJSON file
- Extracted geometry metadata
- Copied file to `.georag/datasets/`
- Registered in `datasets.json`

### Step 3: Build the Index

Generate embeddings and build the retrieval index:

```bash
georag build
```

**Output:**

```
ℹ Building index...

Step 1: Normalizing geometries
ℹ   All datasets already in workspace CRS

Step 2: Validating geometries
ℹ   Fixed 0 invalid geometries

Step 3: Generating embeddings
ℹ   Using embedder: ollama:nomic-embed-text
ℹ   Processing 150 chunks
ℹ   Embedding dimension: 768

Step 4: Finalizing index
ℹ   Index hash: a1b2c3d4e5f6

✓ Index built successfully

Index Information
Hash: a1b2c3d4e5f6
Chunks: 150
Embedding Dimension: 768
Embedder: ollama:nomic-embed-text
```

**What happened:**

- Normalized geometries to workspace CRS
- Validated geometry topology
- Generated embeddings for all features
- Created deterministic index hash
- Saved index state

### Step 4: Query

Ask questions about your data:

```bash
georag query "What cities are in the dataset?"
```

**Output:**

```
Query Plan
Query: What cities are in the dataset?
Spatial Filter: None
Semantic Reranking: Enabled
Top K: 10

Executing Query
ℹ Found 5 spatial matches
ℹ Applying semantic reranking...

Results
ℹ This is a generated answer based on the retrieved spatial features...

Sources

1. cities.geojson (score: 0.95)
  Feature: 1
  This is a sample text excerpt from the first result...

2. cities.geojson (score: 0.87)
  Feature: 2
  Another relevant excerpt from the second result...
```

### Step 5: Check Status

View your workspace status:

```bash
georag status
```

**Output:**

```
Workspace Status
Location: /path/to/my-georag-project
CRS: EPSG:4326
Distance Unit: Meters
Datasets: 1

Index Status
Status: Built
Hash: a1b2c3d4e5f6
Built At: 2024-12-19 12:00:00 UTC
Embedder: ollama:nomic-embed-text
Chunks: 150
```

## Next Steps

### Add More Datasets

```bash
# Add multiple datasets
georag add regions.geojson
georag add boundaries.geojson

# Rebuild index
georag build --force
```

### Spatial Queries

Query with geographic constraints:

```bash
# Query within a bounding box
georag query "What features are here?" \
  --spatial bbox \
  --geometry bbox.geojson

# Query within distance
georag query "What's nearby?" \
  --spatial within \
  --geometry point.geojson \
  --distance 5km
```

### Inspect Your Data

```bash
# List all datasets
georag inspect datasets

# View index details
georag inspect index

# Check CRS information
georag inspect crs

# View configuration
georag inspect config
```

### Use JSON Output

Perfect for scripting:

```bash
# Get JSON output
georag status --json

# Parse with jq
georag status --json | jq '.data.index.built'

# Use in scripts
if georag status --json | jq -e '.data.index.built == true'; then
  echo "Index is ready"
fi
```

### Preview Changes

Use dry-run mode to preview operations:

```bash
# Preview build
georag build --dry-run

# Preview dataset addition
georag add new-dataset.geojson --dry-run
```

## Common Workflows

### Workflow 1: Data Exploration

```bash
# Initialize workspace
georag init exploration

# Add datasets
georag add data/*.geojson

# Build index
georag build

# Explore with queries
georag query "What types of features exist?"
georag query "What's the spatial distribution?"
```

### Workflow 2: Location-Based Search

```bash
# Initialize with appropriate CRS
georag init location-search --crs 4326

# Add location data
georag add locations.geojson

# Build index
georag build

# Query with spatial constraints
georag query "What's within 10km of downtown?" \
  --spatial within \
  --geometry downtown.geojson \
  --distance 10km
```

### Workflow 3: Multi-Dataset Analysis

```bash
# Initialize workspace
georag init analysis

# Add multiple datasets
georag add cities.geojson
georag add roads.geojson
georag add regions.geojson

# Build combined index
georag build

# Query across datasets
georag query "What infrastructure exists in urban areas?" \
  --spatial intersects \
  --geometry urban-areas.geojson
```

## Troubleshooting

### Issue: "Not in a GeoRAG workspace"

**Solution:** Run `georag init` first, or navigate to a directory with a `.georag/` folder.

```bash
# Check if you're in a workspace
ls -la | grep .georag

# If not, initialize
georag init
```

### Issue: "Index not built"

**Solution:** Run `georag build` before querying.

```bash
# Check index status
georag status

# Build if needed
georag build
```

### Issue: "Embedder unavailable"

**Solution:** Ensure Ollama is running and the model is pulled.

```bash
# Check Ollama status
ollama list

# Pull the model if needed
ollama pull nomic-embed-text

# Verify Ollama is running
curl http://localhost:11434/api/tags
```

### Issue: "CRS mismatch"

**Solution:** Either reproject your data or use `--force` flag.

```bash
# Option 1: Use force flag
georag add dataset.geojson --force

# Option 2: Reproject your data externally
# (use GDAL, QGIS, or other GIS tools)

# Option 3: Initialize with matching CRS
georag init --crs 3857
```

### Issue: "Invalid geometry"

**Solution:** GeoRAG will attempt to fix invalid geometries during build.

```bash
# Build with automatic fixes
georag build

# Check build output for fix count
# If fixes fail, validate your data externally
```

## Tips for Success

### 1. Start Small

Begin with a small dataset to understand the workflow:

```bash
# Use a subset of your data
head -n 100 large-dataset.geojson > small-dataset.geojson
georag add small-dataset.geojson
```

### 2. Use Dry-Run

Preview operations before executing:

```bash
# Always preview first
georag build --dry-run

# Then execute
georag build
```

### 3. Check Status Often

Monitor your workspace state:

```bash
# Quick status check
georag status

# Detailed information
georag status --verbose
```

### 4. Leverage JSON Output

Automate with JSON output:

```bash
# Save results
georag query "What's here?" --json > results.json

# Parse results
jq '.data.results' results.json
```

### 5. Keep Index Updated

Rebuild after adding datasets:

```bash
# Add new dataset
georag add new-data.geojson

# Rebuild index
georag build --force
```

## Learning Resources

- **[CLI Reference](cli-reference.md)** - Complete command documentation
- **[Output Formatting](output-formatting.md)** - JSON and dry-run mode
- **[Examples](../examples/)** - Sample code and workflows
- **[Architecture](architecture.md)** - System design (coming soon)
