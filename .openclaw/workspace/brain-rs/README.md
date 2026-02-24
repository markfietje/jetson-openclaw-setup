# Brain Server v6.2

A high-performance semantic search server for OpenClaw, built in Rust and optimized for the Jetson Nano.

## Overview

Brain Server provides local-first, semantic knowledge retrieval using vector embeddings. It powers the RAG (Retrieval-Augmented Generation) pipeline for OpenClaw, enabling the AI to remember and recall information based on meaning rather than keywords.

## Features

- **Semantic Vector Search**: Cosine similarity search across 2,700+ knowledge entries
- **OpenAI-Compatible API**: `/v1/embeddings` endpoint for seamless integration
- **Local-First**: 100% offline, no cloud dependencies
- **NEON SIMD Optimized**: ARM Cortex-A57 vector acceleration
- **Memory Efficient**: ~150MB RAM footprint
- **Fast**: <1ms search latency
- **Deduplication**: Automatic content hashing with xxh3

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    BRAIN SERVER v6.2                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Request → Axum HTTP Server → Tokio Task Pool               │
│                                    ↓                        │
│                           model2vec_rs                      │
│                         (potion-retrieval-32M)              │
│                                    ↓                        │
│                              SQLite                         │
│                        (WAL mode, r2d2 pool)                │
│                                    ↓                        │
│                         NEON SIMD Search                    │
│                       (Cosine Similarity)                   │
│                                    ↓                        │
│                              Results                        │
└─────────────────────────────────────────────────────────────┘
```

## API Endpoints

### OpenAI-Compatible

#### POST /v1/embeddings

Generate embeddings for text using the local model.

**Request:**
```json
{
  "input": "Your text here",
  "model": "minishlab/potion-retrieval-32M"
}
```

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.123, -0.456, ...],
      "index": 0
    }
  ],
  "model": "minishlab/potion-retrieval-32M",
  "usage": {
    "prompt_tokens": 25,
    "total_tokens": 25
  }
}
```

### Brain-Specific Endpoints

#### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "model": "minishlab/potion-retrieval-32M",
  "version": "6.2"
}
```

#### GET /stats

Get database statistics.

**Response:**
```json
{
  "count": 2701,
  "model": "minishlab/potion-retrieval-32M",
  "version": "6.2"
}
```

#### GET /search?q={query}&k={limit}

Semantic search across knowledge base.

**Parameters:**
- `q`: Search query string
- `k`: Number of results (default: 5)

**Response:**
```json
{
  "success": true,
  "results": [
    {
      "id": 123,
      "similarity": 0.85,
      "title": "Example Entry",
      "content": "This is the content..."
    }
  ]
}
```

#### POST /ingest/memory

Ingest new entries from MEMORY.md file.

**Response:**
```json
{
  "success": true,
  "ingested": 5
}
```

#### POST /ingest/rotate

Archive current MEMORY.md and start fresh.

**Response:**
```json
{
  "success": true,
  "archived": "memory_archive/2026-02.md"
}
```

## Building

### Prerequisites

- Rust 1.75+ (stable)
- ARM64 target: `rustup target add aarch64-unknown-linux-gnu`

### Build Script

```bash
./build.sh
```

This will:
1. Compile with ARM NEON optimizations
2. Strip debug symbols
3. Apply UPX compression (if available)
4. Copy binary to workspace root

### Manual Build

```bash
cargo build --release --target aarch64-unknown-linux-gnu
```

## Installation

### Systemd Service

The binary is managed by systemd user service:

```bash
# Check status
systemctl --user status brain-server

# Restart
systemctl --user restart brain-server

# View logs
journalctl --user -u brain-server -f
```

### File Locations

- Binary: `~/.openclaw/workspace/brain-server`
- Source: `~/.openclaw/workspace/brain-rs/`
- Database: `~/.openclaw/workspace/brain.db`
- Memory: `~/.openclaw/workspace/MEMORY.md`
- Archives: `~/.openclaw/workspace/memory_archive/`

## Database Schema

### knowledge table
```sql
CREATE TABLE knowledge (
    id INTEGER PRIMARY KEY,
    title TEXT,
    content TEXT NOT NULL,
    knowledge_type TEXT,
    source TEXT,
    content_hash TEXT UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### embeddings table
```sql
CREATE TABLE embeddings (
    knowledge_id INTEGER PRIMARY KEY,
    vector TEXT,
    FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_knowledge_hash ON knowledge(content_hash);
```

## Performance

| Metric | Value |
|--------|-------|
| **Binary Size** | ~7MB (stripped) |
| **RAM Usage** | ~150MB |
| **Search Latency** | <1ms |
| **Embedding Generation** | ~50ms |
| **Database Entries** | 2,700+ |
| **Vector Dimensions** | 512 |

### Optimizations

- **NEON SIMD**: ARM vector instructions for cosine similarity
- **WAL Mode**: Write-Ahead Logging for SD card longevity
- **Connection Pool**: r2d2 with 4 connections
- **Async Runtime**: Tokio with 2 workers, 4 blocking threads
- **LTO**: Link-time optimization for smaller binary
- **xxh3 Hash**: Fast 64-bit deduplication

## Usage Examples

### Semantic Search

```bash
curl "http://127.0.0.1:8765/search?q=Philippines+visa&k=5"
```

### Generate Embeddings

```bash
curl -X POST http://127.0.0.1:8765/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"input": "Hello world"}'
```

### Auto-Ingest

```bash
# Add to crontab for automatic ingestion
echo "*/5 * * * * curl -s -X POST http://127.0.0.1:8765/ingest/memory" | crontab -
```

## Memory.md Format

The server auto-ingests from a markdown file:

```markdown
# MEMORY.md

## [2026-02-12] Meeting Notes

Content here will be embedded and searchable.

#tag1 #tag2

## [2026-02-11] Technical Note

More content here.
```

## Model Details

- **Model**: minishlab/potion-retrieval-32M
- **Type**: StaticModel (pre-computed embeddings)
- **Parameters**: 32M
- **Output Dimensions**: 512
- **Format**: ONNX Runtime
- **Load Time**: Once at startup, cached in RAM

## Why Rust?

- **Zero GC**: No garbage collection pauses
- **Memory Safety**: No segfaults or data races
- **NEON SIMD**: Auto-vectorization for ARM
- **Small Binary**: Static linking, minimal deps
- **Performance**: Native speed, async/await

## Troubleshooting

### Service Won't Start

```bash
# Check logs
journalctl --user -u brain-server -n 50

# Verify database permissions
ls -la ~/.openclaw/workspace/brain.db

# Test binary directly
~/.openclaw/workspace/brain-server
```

### Build Failures

```bash
# Install dependencies
sudo apt install build-essential libsqlite3-dev

# Clean and rebuild
cargo clean
cargo build --release
```

### Search Returns Empty

```bash
# Check entry count
curl http://127.0.0.1:8765/stats

# Verify embeddings exist
sqlite3 ~/.openclaw/workspace/brain.db "SELECT COUNT(*) FROM embeddings;"

# Re-ingest
curl -X POST http://127.0.0.1:8765/ingest/memory
```

## License

MIT License - Part of the OpenClaw project

## Version History

- **v6.2**: OpenAI-compatible embeddings endpoint
- **v6.1**: Stable release with NEON optimizations
- **v6.0**: Initial Rust rewrite from Python

---

Built for the Jetson Nano. Optimized for local-first AI.
