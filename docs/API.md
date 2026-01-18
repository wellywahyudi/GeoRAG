# GeoRAG REST API Reference

This document describes the GeoRAG HTTP API endpoints.

## Base URL

```
http://localhost:3001
```

The port can be configured via the `GEORAG_PORT` environment variable.

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GEORAG_PORT` | `3001` | HTTP server port |
| `GEORAG_EMBEDDER_MODEL` | `nomic-embed-text` | Ollama embedding model |
| `GEORAG_EMBEDDER_DIM` | `768` | Embedding vector dimensions |
| `OLLAMA_URL` | `http://localhost:11434` | URL for Ollama service |
| `DATABASE_URL` | (none) | PostgreSQL connection string (optional) |

### Storage Backends

The API supports two storage backends:

1. **In-Memory** (default) - Data is ephemeral, suitable for development
2. **PostgreSQL** - Persistent storage, requires `DATABASE_URL` to be set

```bash
# Start with in-memory storage
georag-api

# Start with PostgreSQL
DATABASE_URL=postgresql://user:pass@localhost:5432/georag georag-api
```

---

## Endpoints

### Health Check

Check if the API server is running.

```http
GET /health
```

**Response:**

```json
{
  "status": "ok",
  "service": "georag-api"
}
```

---

## Workspace Management

### Create Workspace

Create a new isolated workspace with specific configuration.

```http
POST /api/v1/workspaces
Content-Type: application/json
```

**Request Body:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | - | Unique workspace name |
| `crs` | integer | No | 4326 | EPSG code for coordinate reference system |
| `distance_unit` | string | No | "Meters" | Unit for distance calculations (Meters, Kilometers, Miles, Feet) |
| `geometry_validity` | string | No | "Lenient" | validation mode (Strict, Lenient) |

**Example:**

```json
{
  "name": "project-alpha",
  "crs": 3857,
  "distance_unit": "Meters"
}
```

**Response (201 Created):**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "project-alpha",
  "crs": 3857,
  "distance_unit": "Meters",
  "geometry_validity": "Lenient",
  "created_at": "2026-01-18T10:00:00Z"
}
```

### List Workspaces

Retrieve all available workspaces.

```http
GET /api/v1/workspaces
```

**Response:**

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "project-alpha",
    "crs": 3857,
    "distance_unit": "Meters",
    "geometry_validity": "Lenient",
    "created_at": "2026-01-18T10:00:00Z"
  }
]
```

### Delete Workspace

Delete a workspace and all its associated data (datasets, index).

```http
DELETE /api/v1/workspaces/:id
```

**Response:**

```json
{
  "success": true,
  "message": "Successfully deleted workspace 550e8400-e29b-41d4-a716-446655440000"
}
```

---

## Workspace Datasets

### List Datasets in Workspace

Retrieve metadata for all datasets in a specific workspace.

```http
GET /api/v1/workspaces/:id/datasets
```

**Response:**

```json
[
  {
    "id": 1,
    "name": "cities.geojson",
    "type": "Point",
    "feature_count": 150,
    "crs": 4326,
    "added_at": "2026-01-18T10:30:00Z"
  }
]
```

### Ingest Dataset (Workspace Scoped)

Upload and ingest a dataset into a specific workspace.

```http
POST /api/v1/ingest
Content-Type: multipart/form-data
```

**Form Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | file | Yes | Dataset file (GeoJSON, GPX, KML, Shapefile, PDF, DOCX) |
| `workspace_id` | string | Yes | UUID of the target workspace |

**Response:**

```json
{
  "success": true,
  "dataset_id": 1,
  "message": "Successfully ingested cities.geojson with 150 features"
}
```

### Delete Dataset

Remove a dataset from a workspace.

```http
DELETE /api/v1/workspaces/:workspace_id/datasets/:dataset_id
```

**Response:**

```json
{
  "success": true,
  "message": "Successfully deleted dataset 1"
}
```

---

## Index Operations

### Get Index Status

Check the status of the search index for a workspace.

```http
GET /api/v1/workspaces/:id/index/status
```

**Response:**

```json
{
  "built": true,
  "rebuilding": false,
  "hash": "a1b2c3d4...",
  "built_at": "2026-01-18T11:00:00Z",
  "chunk_count": 500,
  "embedder": "nomic-embed-text"
}
```

### Rebuild Index

Trigger an asynchronous background job to rebuild the search index.

```http
POST /api/v1/workspaces/:id/index/rebuild
```

**Response (202 Accepted):**

```json
{
  "status": "accepted",
  "message": "Index rebuild started. Poll GET /index/status for progress."
}
```

---

## Querying

### Semantic Search

Execute a spatial-semantic query against a workspace's index.

```http
POST /api/v1/query
Content-Type: application/json
```

**Request Body:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `text` | string | Yes | - | Natural language query text |
| `workspace_id` | string | No | (default) | Target workspace UUID |
| `bbox` | array | No | null | Bounding box filter `[minLng, minLat, maxLng, maxLat]` |
| `top_k` | integer | No | 10 | Maximum number of results to return |

**Example:**

```json
{
  "text": "Find restaurants near beaches",
  "workspace_id": "550e8400-e29b-41d4-a716-446655440000",
  "bbox": [115.0, -8.8, 115.4, -8.4],
  "top_k": 5
}
```

**Response:**

Returns a GeoJSON `FeatureCollection` with query results.

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": { "type": "Point", "coordinates": [...] },
      "properties": {
        "score": 0.92,
        "excerpt": "Beach-side restaurant...",
        "document_path": "restaurants.geojson"
      }
    }
  ]
}
```

---

## Legacy Endpoints (Deprecated)

These endpoints are maintained for backward compatibility but operate only on the default in-memory workspace.

- `GET /api/v1/datasets` - List datasets (default workspace)
- `GET /api/v1/index/integrity` - Get index status (default workspace)
- `POST /api/v1/index/verify` - Verify index (default workspace)

---

## Error Handling

All endpoints return errors in a consistent format:

```json
{
  "error": "Human-readable error message",
  "details": "Technical details (optional)"
}
```

| Code | Meaning |
|------|---------|
| `200` | Success |
| `201` | Created |
| `202` | Accepted (background task started) |
| `400` | Bad Request |
| `404` | Not Found (resource or index missing) |
| `500` | Internal Server Error |
