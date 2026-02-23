# Brain Server v0.8.0 Release Notes

**Release Date:** 2026-02-19
**Platform:** Jetson Nano (ARMv8 Cortex-A57, 4GB RAM)
**Database:** SQLite with r2d2 connection pooling

---

## New Features

### Knowledge Graph

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/ingest/markdown` | POST | Parse markdown content with `[[relation::entity]]` annotations and create entities/relationships |
| `/graph/entity/{name}` | GET | Get entity details and all relationships |
| `/graph/relations` | GET | Get relationships for a specific entity (requires `from` or `to` query param) |
| `/graph/traverse` | GET | Graph traversal with configurable depth (default 2, max 3) |

### Security Enhancements

- **Prompt Injection Detection** - Blocks inputs containing suspicious patterns like `ignore previous`, `system:`, `def `, `import `, `exec(`, etc.
- **Input Validation** - Length limits and character allowlists on all endpoints
- **HTML Escaping** - Titles are escaped before storage
- **SQL Injection Prevention** - All graph endpoints use parameterized queries and validated entity IDs

### Stability Improvements

- **Pool Health Check** - Background task pings DB every 30s to prevent connection timeouts
- **Optimized Pool Settings:**
  - `connection_timeout`: 30s
  - `max_lifetime`: 300s (was 60s)
  - `idle_timeout`: 60s (was 20s)
  - `min_idle`: 2 (was 4)
  - `test_on_check_out`: disabled (was enabled)

---

## API Reference

### `/health` GET
Returns server health status including memory usage and connection pool state.

```bash
curl http://localhost:8765/health
```

### `/ready` GET
Lightweight readiness check.

```bash
curl http://localhost:8765/ready
```

### `/stats` GET
Returns database statistics.

```bash
curl http://localhost:8765/stats | jq '.'
```

**Response:**
```json
{
  "count": 150,
  "embeddings": 150,
  "entities": 42,
  "relationships": 67,
  "model": "minishlab/potion-retrieval-32M",
  "version": "0.8.0"
}
```

### `/add` POST
Add a knowledge chunk with embedding.

```bash
curl -X POST http://localhost:8765/add \
  -H "Content-Type: application/json" \
  -d '{"text": "Your knowledge content here"}'
```

### `/ingest/memory` POST
Ingest memory content with `## [Title]` headers.

```bash
curl -X POST http://localhost:8765/ingest/memory \
  -H "Content-Type: text/plain" \
  -d "## [My Note]
This is the content."
```

### `/search` GET
Semantic search endpoint.

```bash
curl "http://localhost:8765/search?q=your query&k=5"
```

### `/v1/embeddings` POST
Generate embeddings for text.

```bash
curl -X POST http://localhost:8765/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input": "Your text here"}'
```

### `/ingest/markdown` POST (NEW)
Ingest markdown with knowledge graph annotations. **Note:** `title` is required and becomes the source entity for relationships.

```bash
curl -X POST http://localhost:8765/ingest/markdown \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Bignay is [[alternative_to::blueberry]]. It has [[has_property::antioxidants]].",
    "title": "Bignay"
  }'
```

**Relationship mapping:**
- `title` (e.g., "Bignay") → `from_entity` (source)
- Annotation target (e.g., "blueberry") → `to_entity`
- Relation type (e.g., "alternative_to") → `relation_type`

**Response:**
```json
{
  "success": true,
  "id": 42
}
```

### `/graph/entity/{name}` GET (NEW)
Get entity details and relationships.

```bash
curl http://localhost:8765/graph/entity/bignay
```

**Response:**
```json
{
  "id": 1,
  "name": "bignay",
  "entity_type": null,
  "relations": [
    {"to_entity": "blueberry", "relation_type": "alternative_to", "direction": "out"},
    {"to_entity": "antioxidants", "relation_type": "has_property", "direction": "out"}
  ]
}
```

### `/graph/relations` GET (NEW)
Get relationships for an entity.

```bash
curl "http://localhost:8765/graph/relations?from=bignay"
# or
curl "http://localhost:8765/graph/relations?to=antioxidants"
```

### `/graph/traverse` GET (NEW)
Traverse the knowledge graph.

```bash
curl "http://localhost:8765/graph/traverse?entity=bignay&depth=2"
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_HOST` | `127.0.0.1` | Host to bind to |
| `BIND_PORT` | `8765` | Port to listen on |

---

## Build Instructions

```bash
# Clone and build (Jetson Nano optimized)
cd ~/.openclaw/workspace/brain-rs

# Build with size optimization for minimal RAM footprint
# Note: Uses opt-level="z" and LTO - build may use significant RAM
cargo build --release

# Run
./target/release/brain-server

# Or with custom port
BIND_PORT=8765 ./target/release/brain-server
```

**Build Profile:** `opt-level = "z"` for smallest binary and lowest RAM usage. ARM Cortex-A57 optimized with NEON SIMD flags.

---

## Database Schema

### entities (NEW)
```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    entity_type TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_entities_name ON entities(name);
CREATE INDEX idx_entities_type ON entities(entity_type);
```

### relationships (NEW)
```sql
CREATE TABLE relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity_id INTEGER NOT NULL,
    to_entity_id INTEGER NOT NULL,
    relation_type TEXT NOT NULL,
    knowledge_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(from_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
    FOREIGN KEY(to_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
    FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE SET NULL
);
CREATE INDEX idx_rels_from ON relationships(from_entity_id);
CREATE INDEX idx_rels_to ON relationships(to_entity_id);
CREATE UNIQUE INDEX idx_rels_unique ON relationships(from_entity_id, to_entity_id, relation_type);
```

---

## Annotation Syntax

Use `[[relation::entity]]` syntax in markdown content:

```
Bignay is [[alternative_to::blueberry]].
It has [[has_property::antioxidants]].
Grows in [[located_in::southeast_asia]].
```

---

## Changelog

### v0.8.0 (2026-02-19)
- Added knowledge graph tables (entities, relationships)
- Added `/ingest/markdown` endpoint for KG ingestion
- Added `/graph/entity/:name`, `/graph/relations`, `/graph/traverse` endpoints
- Added prompt injection detection
- Added pool health check background task
- Optimized r2d2 pool settings for stability
- Updated sysinfo to 0.38.2

**Bug Fixes:**
- Fixed SQL injection vulnerability in `/graph/traverse` - now uses validated entity IDs
- Fixed inverted relationship logic in `/ingest/markdown` - title correctly maps to `from_entity`, annotation target to `to_entity`
- `/ingest/markdown` now requires `title` field (used as source entity for relationships)

**Build Optimizations:**
- Added ARM Cortex-A57 specific compiler flags with NEON/SIMD support
- Enabled `opt-level = "z"` for minimal binary size and RAM usage
- Full LTO enabled for dead code elimination
