# Output Formatting Guide

GeoRAG CLI supports flexible output formatting to accommodate different use cases, from interactive terminal usage to automated scripting and CI/CD pipelines.

## Table of Contents

- [Overview](#overview)
- [JSON Output Mode](#json-output-mode)
- [Dry-Run Mode](#dry-run-mode)
- [Combining Modes](#combining-modes)
- [Command-Specific Output](#command-specific-output)
- [Examples](#examples)

## Overview

GeoRAG provides two global flags that affect command output:

- `--json`: Output results in machine-readable JSON format
- `--dry-run`: Display planned actions without executing them

These flags can be used independently or together, and work with all state-modifying commands.

## JSON Output Mode

### Purpose

JSON output mode is designed for:

- **Automation**: Parse command results in scripts and CI/CD pipelines
- **Integration**: Connect GeoRAG with other tools and services
- **Programmatic Access**: Build applications on top of GeoRAG CLI

### Usage

Add the `--json` flag to any command:

```bash
georag init --json
georag add dataset.geojson --json
georag build --json
georag status --json
```

### Output Structure

All JSON output follows a consistent structure:

```json
{
  "status": "success",
  "data": {
    // Command-specific data
  }
}
```

For errors:

```json
{
  "status": "error",
  "message": "Error description"
}
```

### Command-Specific JSON Output

#### Init Command

```json
{
  "status": "success",
  "data": {
    "workspace_path": "/path/to/workspace",
    "crs": 4326,
    "distance_unit": "Meters",
    "validity_mode": "Lenient"
  }
}
```

#### Add Command

```json
{
  "status": "success",
  "data": {
    "dataset_name": "my-dataset",
    "geometry_type": "Polygon",
    "feature_count": 150,
    "crs": 4326,
    "crs_mismatch": null
  }
}
```

With CRS mismatch:

```json
{
  "status": "success",
  "data": {
    "dataset_name": "my-dataset",
    "geometry_type": "Polygon",
    "feature_count": 150,
    "crs": 3857,
    "crs_mismatch": {
      "dataset_crs": 3857,
      "workspace_crs": 4326
    }
  }
}
```

#### Build Command

```json
{
  "status": "success",
  "data": {
    "index_hash": "a1b2c3d4e5f6",
    "chunk_count": 500,
    "embedding_dim": 768,
    "embedder": "ollama:nomic-embed-text",
    "normalized_count": 2,
    "fixed_count": 0
  }
}
```

#### Query Command

```json
{
  "status": "success",
  "data": {
    "query": "What are the main features?",
    "spatial_matches": 5,
    "results": [
      {
        "content": "Excerpt from the document...",
        "source": "dataset-1.geojson",
        "score": 0.95
      }
    ],
    "explanation": "Spatial Phase: 15 features evaluated, 5 matched..."
  }
}
```

#### Status Command

```json
{
  "status": "success",
  "data": {
    "workspace_path": "/path/to/workspace",
    "crs": 4326,
    "distance_unit": "Meters",
    "dataset_count": 3,
    "index": {
      "built": true,
      "hash": "a1b2c3d4e5f6",
      "built_at": "2024-12-19T12:00:00Z",
      "embedder": "ollama:nomic-embed-text",
      "chunk_count": 500,
      "embedding_dim": 768
    },
    "storage": null
  }
}
```

#### Inspect Commands

**Datasets:**

```json
{
  "status": "success",
  "data": {
    "datasets": [
      {
        "id": 1,
        "name": "my-dataset",
        "geometry_type": "Polygon",
        "feature_count": 150,
        "crs": 4326,
        "added_at": "2024-12-19T12:00:00Z"
      }
    ]
  }
}
```

**Index:**

```json
{
  "status": "success",
  "data": {
    "built": true,
    "hash": "a1b2c3d4e5f6",
    "built_at": "2024-12-19T12:00:00Z",
    "embedder": "ollama:nomic-embed-text",
    "chunk_count": 500,
    "embedding_dim": 768
  }
}
```

**CRS:**

```json
{
  "status": "success",
  "data": {
    "workspace_crs": 4326,
    "datasets": [
      {
        "name": "my-dataset",
        "crs": 4326,
        "matches_workspace": true
      }
    ]
  }
}
```

**Config:**

```json
{
  "status": "success",
  "data": {
    "crs": {
      "value": 4326,
      "source": "File"
    },
    "distance_unit": {
      "value": "Meters",
      "source": "File"
    },
    "geometry_validity": {
      "value": "Lenient",
      "source": "File"
    },
    "embedder": {
      "value": "ollama:nomic-embed-text",
      "source": "Default"
    }
  }
}
```

## Dry-Run Mode

### Purpose

Dry-run mode allows you to preview what a command will do before actually executing it. This is useful for:

- **Safety**: Verify operations before making changes
- **Planning**: Understand the impact of a command
- **Documentation**: Generate action plans for review

### Usage

Add the `--dry-run` flag to state-modifying commands:

```bash
georag init --dry-run
georag add dataset.geojson --dry-run
georag build --dry-run
```

### Behavior

In dry-run mode:

- ✓ Command validates all inputs
- ✓ Displays all planned actions with details
- ✗ No files are created or modified
- ✗ No state changes occur

### Output Format

Human-readable format:

```
Planned Actions (Dry Run)
ℹ 1. CreateDirectory: Create .georag directory at /path/to/workspace
ℹ 2. CreateFile: Create config.toml
ℹ    - CRS: EPSG:4326
ℹ    - Distance Unit: Meters
ℹ    - Validity Mode: Lenient
ℹ 3. CreateDirectory: Create datasets directory
ℹ 4. CreateDirectory: Create index directory
ℹ 5. CreateFile: Create datasets.json (empty)
ℹ
No changes were made. Run without --dry-run to execute these actions.
```

## Combining Modes

You can combine `--json` and `--dry-run` for programmatic dry-run output:

```bash
georag init --dry-run --json
```

Output:

```json
{
  "status": "success",
  "data": {
    "dry_run": true,
    "planned_actions": [
      {
        "action_type": "create_directory",
        "description": "Create .georag directory at /path/to/workspace",
        "details": []
      },
      {
        "action_type": "create_file",
        "description": "Create config.toml",
        "details": [
          "CRS: EPSG:4326",
          "Distance Unit: Meters",
          "Validity Mode: Lenient"
        ]
      }
    ]
  }
}
```

### Action Types

Dry-run mode uses the following action types:

- `create_directory`: Creating a new directory
- `create_file`: Creating a new file
- `write_file`: Writing content to a file
- `copy_file`: Copying a file
- `modify_file`: Modifying an existing file

## Command-Specific Output

### Commands Supporting JSON Output

All commands support JSON output:

- ✓ `init`
- ✓ `add`
- ✓ `build`
- ✓ `query`
- ✓ `inspect` (all subcommands)
- ✓ `status`

### Commands Supporting Dry-Run

Only state-modifying commands support dry-run:

- ✓ `init`
- ✓ `add`
- ✓ `build`
- ✗ `query` (read-only)
- ✗ `inspect` (read-only)
- ✗ `status` (read-only)

## Examples

### Example 1: Scripting with JSON Output

```bash
#!/bin/bash

# Initialize workspace and capture output
output=$(georag init /path/to/workspace --json)

# Parse JSON to extract CRS
crs=$(echo "$output" | jq -r '.data.crs')

echo "Workspace initialized with CRS: EPSG:$crs"
```

### Example 2: Safe Operations with Dry-Run

```bash
# Preview what will happen
georag build --dry-run

# Review the output, then execute
georag build
```

### Example 3: CI/CD Integration

```bash
#!/bin/bash
set -e

# Initialize workspace
georag init . --json > init-result.json

# Add datasets
for dataset in data/*.geojson; do
  georag add "$dataset" --json >> add-results.json
done

# Build index
georag build --json > build-result.json

# Verify build succeeded
if jq -e '.status == "success"' build-result.json > /dev/null; then
  echo "Build successful"
  exit 0
else
  echo "Build failed"
  exit 1
fi
```

### Example 4: Automated Testing

```bash
# Test dry-run doesn't modify state
initial_state=$(ls -R .georag)
georag build --dry-run
final_state=$(ls -R .georag)

if [ "$initial_state" = "$final_state" ]; then
  echo "✓ Dry-run test passed"
else
  echo "✗ Dry-run test failed: state was modified"
  exit 1
fi
```

### Example 5: Monitoring and Logging

```bash
# Log all operations in JSON format
georag init . --json | tee -a operations.log
georag add dataset.geojson --json | tee -a operations.log
georag build --json | tee -a operations.log

# Parse logs for analysis
jq -s '[.[] | select(.status == "error")]' operations.log
```

## Best Practices

### When to Use JSON Output

- ✓ Scripting and automation
- ✓ CI/CD pipelines
- ✓ Integration with other tools
- ✓ Programmatic parsing of results
- ✗ Interactive terminal usage (use human-readable format)

### When to Use Dry-Run

- ✓ Before making significant changes
- ✓ When learning the tool
- ✓ In production environments
- ✓ For generating documentation
- ✗ In automated scripts (unless intentional)

### Error Handling

Always check the `status` field in JSON output:

```bash
result=$(georag build --json)
status=$(echo "$result" | jq -r '.status')

if [ "$status" != "success" ]; then
  error=$(echo "$result" | jq -r '.message')
  echo "Error: $error"
  exit 1
fi
```

### Combining with Other Flags

Output formatting flags work with all other flags:

```bash
# Dry-run with custom CRS
georag init --crs 3857 --dry-run

# JSON output with verbose status
georag status --verbose --json

# Dry-run with force flag
georag add dataset.geojson --force --dry-run
```

## Troubleshooting

### JSON Parsing Errors

If you encounter JSON parsing errors:

1. Ensure you're redirecting stderr: `2>/dev/null`
2. Check for compilation warnings mixed with output
3. Use `jq` to validate JSON: `georag init --json | jq .`

### Dry-Run Not Working

If dry-run appears to modify state:

1. Verify you're using the `--dry-run` flag
2. Check that the command supports dry-run (state-modifying only)
3. Ensure no other processes are modifying the workspace

### Missing Fields in JSON

If expected fields are missing:

1. Check the command-specific output structure
2. Verify the command completed successfully
3. Use `--verbose` flag for additional details (where supported)

## See Also

- [CLI Reference](cli-reference.md) - Complete command reference
- [Configuration Guide](configuration.md) - Configuration management
- [API Documentation](api.md) - Programmatic API access
