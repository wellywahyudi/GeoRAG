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

### Query

Execute a spatial-semantic query against the indexed data.

```http
POST /api/v1/query
Content-Type: application/json
```

**Request Body:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `text` | string | Yes | - | Natural language query text |
| `bbox` | array | No | null | Bounding box filter `[minLng, minLat, maxLng, maxLat]` |
| `top_k` | integer | No | 10 | Maximum number of results to return |

**Example Request:**

```json
{
  "text": "Find restaurants near beaches",
  "bbox": [115.0, -8.8, 115.4, -8.4],
  "top_k": 5
}
```

**Response:**

Returns a GeoJSON `FeatureCollection` with query results:

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [115.234, -8.567]
      },
      "properties": {
        "score": 0.92,
        "excerpt": "Beach-side restaurant offering seafood...",
        "document_path": "restaurants.geojson",
        "chunk_id": 42,
        "feature_id": 15
      }
    }
  ]
}
```

**Properties:**

| Property | Type | Description |
|----------|------|-------------|
| `score` | float | Relevance score (0.0 - 1.0) |
| `excerpt` | string | Relevant text excerpt from the source |
| `document_path` | string | Source document filename |
| `chunk_id` | integer | Internal chunk identifier |
| `feature_id` | integer | Associated spatial feature ID |
| `page` | integer | Page number (for PDF documents) |

**Error Response:**

```json
{
  "error": "Query execution failed",
  "details": "Embedder unavailable: connection refused"
}
```

---

### List Datasets

Retrieve all registered datasets.

```http
GET /api/v1/datasets
```

**Response:**

```json
[
  {
    "id": "cities",
    "type": "Point",
    "count": 150
  },
  {
    "id": "roads",
    "type": "LineString",
    "count": 2500
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Dataset name/identifier |
| `type` | string | Geometry type (Point, LineString, Polygon, etc.) |
| `count` | integer | Number of features in the dataset |

---

### Ingest Dataset

Upload and ingest a geospatial dataset file.

```http
POST /api/v1/ingest
Content-Type: multipart/form-data
```

**Form Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | file | Yes | Dataset file (GeoJSON, GPX, KML, Shapefile, PDF, DOCX) |

**Supported Formats:**

- GeoJSON (`.geojson`, `.json`)
- GPX (`.gpx`)
- KML (`.kml`)
- Shapefile (`.shp` + `.dbf`, `.shx`)
- PDF (`.pdf`) - Document with optional spatial association
- DOCX (`.docx`) - Document with optional spatial association

**Example (curl):**

```bash
curl -X POST http://localhost:3001/api/v1/ingest \
  -F "file=@cities.geojson"
```

**Success Response:**

```json
{
  "success": true,
  "dataset_id": 1,
  "message": "Successfully ingested cities.geojson with 150 features"
}
```

**Error Response:**

```json
{
  "success": false,
  "message": "Failed to parse file",
  "details": "Invalid GeoJSON: expected object at line 1"
}
```

---

### Index Integrity

Get the current index state and integrity hash.

```http
GET /api/v1/index/integrity
```

**Response:**

```json
{
  "hash": "a1b2c3d4e5f6",
  "built_at": "2026-01-16T12:00:00Z",
  "embedder": "nomic-embed-text",
  "chunk_count": 500,
  "embedding_dim": 768
}
```

| Field | Type | Description |
|-------|------|-------------|
| `hash` | string | Deterministic hash of index state |
| `built_at` | string | ISO 8601 timestamp of index build |
| `embedder` | string | Embedding model used |
| `chunk_count` | integer | Total chunks in index |
| `embedding_dim` | integer | Embedding vector dimensions |

**Error Response (Index Not Built):**

```json
{
  "error": "Index not found",
  "details": "Index has not been built yet"
}
```

---

### Verify Index Integrity

Recompute the index hash and verify it matches the stored value.

```http
POST /api/v1/index/verify
```

**Response:**

```json
{
  "stored_hash": "a1b2c3d4e5f6",
  "computed_hash": "a1b2c3d4e5f6",
  "matches": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `stored_hash` | string | Hash stored when index was built |
| `computed_hash` | string | Freshly computed hash from current data |
| `matches` | boolean | Whether the hashes match (integrity verified) |

**Use Cases:**

- Verify index hasn't been corrupted
- Detect if underlying data has changed
- Continuous integrity monitoring

---

## Error Handling

All endpoints return errors in a consistent format:

```json
{
  "error": "Human-readable error message",
  "details": "Technical details (optional)"
}
```

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `400` | Bad Request (invalid input) |
| `404` | Not Found (index not built, etc.) |
| `500` | Internal Server Error |

---

## CORS

The API enables CORS for `http://localhost:3000` by default, allowing:

- Methods: `GET`, `POST`, `OPTIONS`
- Headers: `Content-Type`, `Authorization`

---

## Authentication

The current API version does not require authentication. Future versions may add:

- API key authentication
- JWT bearer tokens
- OAuth2 integration

---

## Rate Limiting

No rate limiting is currently enforced. For production deployments, consider adding a reverse proxy with rate limiting.

---

## Examples

### Complete Workflow

```bash
# 1. Start the API server
DATABASE_URL=postgresql://localhost/georag georag-api &

# 2. Upload a dataset
curl -X POST http://localhost:3001/api/v1/ingest \
  -F "file=@data/cities.geojson"

# 3. Query the data
curl -X POST http://localhost:3001/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "text": "What are the largest cities?",
    "top_k": 5
  }'

# 4. Check index integrity
curl http://localhost:3001/api/v1/index/integrity
```

### JavaScript/TypeScript

```typescript
// Query the API
const response = await fetch('http://localhost:3001/api/v1/query', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    text: 'Find parks near downtown',
    bbox: [-122.5, 37.7, -122.3, 37.9],
    top_k: 10
  })
});

const geojson = await response.json();
console.log(`Found ${geojson.features.length} results`);
```

### Python

```python
import requests

# Query with spatial filter
response = requests.post(
    'http://localhost:3001/api/v1/query',
    json={
        'text': 'Historic landmarks',
        'bbox': [-74.1, 40.6, -73.9, 40.8],
        'top_k': 20
    }
)

results = response.json()
for feature in results['features']:
    print(f"{feature['properties']['score']:.2f}: {feature['properties']['excerpt'][:50]}...")
```

---

## OpenAPI Specification

An OpenAPI 3.0 specification is planned for future releases. This will enable:

- Auto-generated client libraries
- Interactive API documentation (Swagger UI)
- API testing tools

---

## Changelog

### v0.1.0

- Initial API release
- Query endpoint with spatial filtering
- Dataset listing and ingestion
- Index integrity endpoints

---

## See Also

- [Documentation](README.md)
- [CLI Reference](CLI.md)
- [Architecture Overview](README.md#architecture)
