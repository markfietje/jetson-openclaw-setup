# Brain Server v6.2 - Quick Reference

## Endpoints

### Health & Status

```bash
# Health check
GET /health

# Database stats
GET /stats

# Response: {"count": 243, "model": "minishlab/potion-retrieval-32M", "embedding_dim": 512}
```

### Add Knowledge

```bash
# POST /add - Add text with automatic embedding
POST /add
Content-Type: application/json

{
  "text": "Your text here",
  "title": "Optional title",
  "metadata": {"key": "value"}  // Optional JSON
}

# Response
{
  "success": true,
  "id": 123,
  "error": null
}
```

### Search

```bash
# POST /search - Semantic vector search (NEW)
POST /search
Content-Type: application/json

{
  "query": "Search query",
  "k": 10,                    // Number of results (default: 10)
  "truncate_dims": 256        // Optional: 512, 256, 128, 64, 32
}

# Response
{
  "success": true,
  "results": [
    {
      "id": 123,
      "text": "Matching text...",
      "title": "Title",
      "distance": 0.234,       // 0 = identical, 1 = unrelated
      "created_at": "2026-02-10 22:30:00"
    }
  ],
  "query_time_ms": 3
}
```

### Legacy Search

```bash
# GET /search - Backward compatible
GET /search?q=query&k=5

# Response (similar to POST but with "similarity" instead of "distance")
{
  "success": true,
  "results": [
    {
      "id": 123,
      "title": "Title",
      "content": "Text...",
      "similarity": 0.766,    // 1 = identical, 0 = unrelated
      "created_at": "2026-02-10 22:30:00"
    }
  ]
}
```

### Ingest

```bash
# POST /ingest/memory - Auto-ingest from MEMORY.md
POST /ingest/memory

# Response
{
  "success": true,
  "ingested": 3
}
```

## curl Examples

```bash
# Add goat farming knowledge
curl -X POST http://127.0.0.1:8765/add \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Goats need 15-20 sq ft of shelter space per animal.",
    "title": "Goat Housing Basics"
  }'

# Search for goat info
curl -X POST http://127.0.0.1:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How much space for goats?",
    "k": 5
  }'

# Fast search with 256 dims
curl -X POST http://127.0.0.1:8765/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "tile preparation",
    "k": 10,
    "truncate_dims": 256
  }'

# Legacy search
curl "http://127.0.0.1:8765/search?q=goat%20housing&k=5"

# Check stats
curl http://127.0.0.1:8765/stats

# Ingest MEMORY.md
curl -X POST http://127.0.0.1:8765/ingest/memory
```

## Distance vs Similarity

**POST /search returns `distance`:**
- `0.0` = identical meaning
- `0.5` = somewhat related
- `1.0` = unrelated
- `>1.0` = opposite meaning

**GET /search returns `similarity`:**
- `1.0` = identical meaning
- `0.5` = somewhat related
- `0.0` = unrelated
- `<0.0` = opposite meaning

**Conversion:** `similarity = 1.0 - (distance / 2.0)`

## Matryoshka Truncation

Reduce embedding dimensions for faster searches:

| Dims | Quality | Speed | Use Case |
|------|--------|-------|----------|
| 512 | 100% | 5ms | Best quality |
| 256 | 90% | 3ms | **Recommended** |
| 128 | 75% | 2ms | Fast rough match |
| 64 | 50% | 1ms | Very fast |

## Database Schema

```sql
-- Chunks table
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    text TEXT NOT NULL,
    title TEXT,
    metadata TEXT,
    content_hash TEXT UNIQUE,
    created_at TEXT
);

-- Vector table with HNSW index
CREATE VIRTUAL TABLE vec_chunks
USING vec0(
    id INTEGER PRIMARY KEY,
    embedding float[512]
) WITH hnsw(max_elements=100000, m=16, ef_construction=200);
```

## Performance

On Jetson Nano (4GB, Cortex-A57):

| Operation | Time | Notes |
|-----------|------|-------|
| Add chunk | 50-100ms | Includes embedding |
| Search (512d) | 3-5ms | Full precision |
| Search (256d) | 2-3ms | **Recommended** |
| Search (128d) | 1-2ms | Fast rough match |
| Ingest MEMORY.md | 2-5s | Depends on entries |

## Error Codes

| Error | Meaning | Solution |
|-------|---------|----------|
| `Duplicate content` | Content hash already exists | Normal - prevents duplicates |
| `Failed to generate embedding` | Model error | Check model is loaded |
| `Failed to get DB connection` | Connection pool exhausted | Increase pool size or retry |
| `Task join failed` | Thread panic | Check logs |

## Troubleshooting

```bash
# Check service status
systemctl --user status brain-server

# View logs
journalctl --user -u brain-server -f

# Restart service
systemctl --user restart brain-server

# Check database
sqlite3 ~/.openclaw/workspace/brain.db "SELECT COUNT(*) FROM chunks;"

# Verify embeddings
sqlite3 ~/.openclaw/workspace/brain.db "SELECT COUNT(*) FROM vec_chunks;"

# Test manually
cd ~/.openclaw/workspace/brain-rs
./test.sh
```

## Build & Deploy

```bash
# Build optimized binary
cd ~/.openclaw/workspace/brain-rs
./build.sh

# Update service
systemctl --user stop brain-server
cp target/aarch64-unknown-linux-gnu/release/brain-server ~/.openclaw/workspace/brain-server
systemctl --user start brain-server

# Verify
curl http://127.0.0.1:8765/health
```

## Quick Test

```bash
# Add something
curl -X POST http://127.0.0.1:8765/add \
  -H "Content-Type: application/json" \
  -d '{"text":"Test entry"}'

# Search for it
curl -X POST http://127.0.0.1:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query":"test","k":5}'
```

## Memory Wrapper

The `mem` command still works (legacy search):

```bash
# Search using mem wrapper
~/.openclaw/workspace/mem "your search query"
```

## Version Info

```bash
# Check version
curl http://127.0.0.1:8765/health
# Response: {"status":"ok","model":"minishlab/potion-retrieval-32M","version":"0.6.1-vec"}
```

---

**Full docs:** See `README.md` and `MIGRATION.md`
