# Brain Server v0.8.0 Technical Specification

> **Self-hosted, lightweight memory & knowledge graph server for AI agents**

---

## Overview

Brain Server is a Rust-based, self-hosted memory and knowledge graph service designed for AI assistants. It combines semantic vector search with explicit relationship extraction, all running locally on minimal hardware (tested on Jetson Nano with 4GB RAM).

**Version:** 0.8.0  
**Architecture:** REST API + SQLite + Vector Embeddings + Knowledge Graph  
**Model:** minishlab/potion-retrieval-32M (512 dimensions)  
**Platform:** Linux (ARM64/x86_64)

---

## Core Features

### 1. Semantic Vector Search

| Feature | Description |
|---------|-------------|
| **Embedding Model** | potion-retrieval-32M (32M params, 512 dims) |
| **Storage** | SQLite with JSON vectors |
| **Similarity** | Cosine similarity |
| **Chunk Size** | ~400 tokens with 80-token overlap |
| **Indexing** | Content hash-based deduplication |

**Endpoints:**
- `GET /search?q=query&k=5` — Semantic search
- `POST /add` — Add single entry
- `POST /ingest/memory` — Batch ingest from markdown

---

### 2. Knowledge Graph (NEW in v0.8.0)

| Feature | Description |
|---------|-------------|
| **Storage** | SQLite (entities + relationships tables) |
| **Annotation Format** | `[[relation::entity]]` wiki-style |
| **Relation Types** | alternative_to, related_to, has_property, best_for, source_from, manages, is_a |
| **Traversal** | Multi-hop graph queries (depth 1-3) |
| **Deduplication** | Unique constraints on entities and relationships |

**Endpoints:**
- `POST /ingest/markdown` — Parse and store markdown with graph annotations
- `GET /graph/entity/:name` — Get entity + all relations
- `GET /graph/relations?from=entity` — Query relationships
- `GET /graph/traverse?entity=name&depth=2` — Graph traversal

---

### 3. Embedding Generation

| Feature | Description |
|---------|-------------|
| **Model** | model2vec-rs (Rust native) |
| **Dimensions** | 512 |
| **Performance** | ~1ms per embedding on CPU |
| **Endpoint** | `POST /v1/embeddings` |

---

### 4. System Features

| Feature | Description |
|---------|-------------|
| **Database** | SQLite with WAL mode, connection pooling |
| **Health Checks** | `/health`, `/ready`, `/stats` |
| **CORS** | Enabled for all origins |
| **Async** | Axum + Tokio (spawn_blocking for CPU work) |
| **Graceful Shutdown** | 10-second drain |

---

## API Reference

### Search & Retrieval

```bash
# Semantic search
GET /search?q=blueberry+alternatives&k=5

# Response:
{
  "success": true,
  "results": [
    {
      "id": 1,
      "title": "Philippine Superfruits",
      "content": "Bignay is...",
      "similarity": 0.85
    }
  ]
}
```

### Knowledge Graph

```bash
# Ingest markdown with annotations
POST /ingest/markdown
{
  "content": "Bignay is [[alternative_to::blueberry]]. It has [[has_property::anthocyanins]].",
  "title": "Bignay"
}

# Get entity
GET /graph/entity/bignay

# Response:
{
  "id": 1,
  "name": "bignay",
  "entity_type": null,
  "relations": [
    {"to_entity": "blueberry", "relation_type": "alternative_to", "direction": "out"},
    {"to_entity": "anthocyanins", "relation_type": "has_property", "direction": "out"}
  ]
}

# Traverse graph
GET /graph/traverse?entity=bignay&depth=2

# Get stats (now includes graph)
GET /stats

{
  "count": 380,
  "embeddings": 380,
  "entities": 156,
  "relationships": 203,
  "model": "minishlab/potion-retrieval-32M",
  "version": "0.8.0"
}
```

---

## Database Schema

### Existing Tables

```sql
-- Knowledge entries
CREATE TABLE knowledge (
    id INTEGER PRIMARY KEY,
    title TEXT,
    content TEXT NOT NULL,
    knowledge_type TEXT,
    source TEXT DEFAULT 'manual',
    content_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Vector embeddings
CREATE TABLE embeddings (
    knowledge_id INTEGER PRIMARY KEY,
    vector TEXT,  -- JSON [f32, ...]
    FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE CASCADE
);
```

### NEW in v0.8.0

```sql
-- Entities
CREATE TABLE entities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    entity_type TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_entities_name ON entities(name);
CREATE INDEX idx_entities_type ON entities(entity_type);

-- Relationships
CREATE TABLE relationships (
    id INTEGER PRIMARY KEY,
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

## Performance

| Metric | Value |
|--------|-------|
| **Embedding Speed** | ~1ms per query |
| **Search Speed** | <10ms for 380 entries |
| **Memory Usage** | ~1MB for 1K entries |
| **Storage** | ~2KB per entry (vector + text) |
| **Cold Start** | ~3 seconds |
| **Concurrent Requests** | Up to 10 (pooled) |

---

## Hugo Integration

Brain Server works seamlessly with Hugo static site generator:

```markdown
---
title: "Philippine Superfruits"
---

# Philippine Superfruits

Bignay is [[alternative_to::blueberry]] with [[has_property::anthocyanins]].

Related fruits: [[related_to::duhat]], [[related_to::mulberry]]
```

**Workflow:**
1. Write Hugo post with `[[ ]]` annotations
2. Hugo builds the site (ignores annotations)
3. Brain Server ingests markdown → extracts graph
4. Query both semantic + relationship data!

---

## Comparison with Commercial Services

### Brain Server vs. Paid Services

| Feature | Brain Server | Mem0 | Pinecone | Weaviate | Chroma |
|---------|--------------|------|----------|----------|--------|
| **Type** | Self-hosted | Cloud/Self | Cloud | Cloud/Self | Cloud/Self |
| **Vector Search** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Knowledge Graph** | ✅ | ✅ | ❌ | ✅ (limited) | ❌ |
| **Markdown Annotations** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Embedding Model** | Built-in (32M) | External | External | External | External |
| **Cost** | **$0** | $$ | $$$ | $$ | Free tier |
| **RAM Usage** | ~1GB | Cloud | Cloud | 2GB+ | 500MB |
| **CPU Only** | ✅ | Cloud | Cloud | ❌ | ❌ |
| **ARM64 Support** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Data Privacy** | 100% local | Third-party | Third-party | Third-party | Third-party |

---

### Why Brain Server is Better

#### 1. **Cost: $0 vs. $50-500/month**

| Service | Monthly Cost |
|---------|--------------|
| **Brain Server** | **$0** (one-time hardware) |
| Mem0 (Pro) | $49/month |
| Pinecone (Standard) | $70/month |
| Weaviate (Cloud) | $75/month |
| Qdrant (Cloud) | $45/month |

For the cost of one month of Pinecone, you can buy a Jetson Nano that will run Brain Server for years!

---

#### 2. **100% Data Privacy**

| Service | Data Leaves Your Machine? |
|---------|--------------------------|
| **Brain Server** | **No** — everything stays local |
| Mem0 | Yes — sent to cloud |
| Pinecone | Yes — sent to cloud |
| Weaviate (Cloud) | Yes — sent to cloud |

Your personal data, memories, and knowledge never leave your infrastructure.

---

#### 3. **Works on Minimal Hardware**

| Service | Minimum Requirements |
|---------|---------------------|
| **Brain Server** | **4GB RAM, any Linux** (tested on Jetson Nano) |
| Weaviate | 8GB RAM recommended |
| Qdrant | 4GB (but slower) |
| Chroma | 2GB (but limited features) |

Brain Server runs on a $150 Jetson Nano. Other services require cloud VMs starting at $20/month.

---

#### 4. **Native Knowledge Graph**

Most vector databases only do similarity search. Brain Server adds:

- **Explicit relationships** via `[[relation::entity]]` syntax
- **Graph traversal** — find all paths between entities
- **Entity queries** — "what relates to X?"
- **Hugo integration** — blog posts become queryable knowledge bases

---

#### 5. **No External Dependencies**

| Service | Requires |
|---------|----------|
| **Brain Server** | **Nothing** — self-contained |
| Mem0 | API key, internet |
| Pinecone | API key, internet |
| Weaviate | Docker/Cloud |
| Chroma | Python, internet |

Brain Server runs on bare metal, no Docker required.

---

### When to Use Paid Services

| Use Case | Recommendation |
|----------|----------------|
| **Personal AI assistant** | Brain Server ✅ |
| **Startup MVP** | Brain Server ✅ |
| **Enterprise (100M+ vectors)** | Pinecone/Weaviate |
| **Team collaboration (cloud)** | Mem0 |
| **Need managed infrastructure** | Pinecone |

---

## Installation

### Build from Source

```bash
# Clone
git clone https://github.com/your-repo/brain-rs.git
cd brain-rs

# Build
cargo build --release

# Run
./target/release/brain-server
```

### Docker (optional)

```yaml
version: '3.8'
services:
  brain:
    build: .
    ports:
      - "8765:8765"
    volumes:
      - ./data:/root/.openclaw/workspace
```

---

## Roadmap

### v0.9.0 (Planned)
- [ ] Recursive graph traversal (SQL CTE)
- [ ] Entity types inference
- [ ] Batch file ingestion endpoint
- [ ] File watcher for auto-ingest

### v1.0.0 (Goals)
- [ ] Relationship inference from plain text (LLM)
- [ ] Graph visualization endpoint
- [ ] Multi-hop queries with path finding
- [ ] Plugin system for custom extractors

---

## Use Cases

### 1. Personal AI Memory
- Store conversations, notes, preferences
- Query: "what did I tell you about Alice?"
- Relationship: "Alice → manages → auth"

### 2. Knowledge Base for Projects
- Ingest documentation, READMEs, specs
- Query: "how does auth work?"
- Graph shows related components

### 3. Hugo Blog Enhancement
- Write posts with `[[ ]]` annotations
- Automatic knowledge graph from blog
- Q&A bot over your blog content

### 4. Research Paper Assistant
- Ingest papers as markdown
- Extract entities (authors, papers, concepts)
- Query: "what has Alice written about neural networks?"

---

## Technical Stack

| Component | Library | Version |
|-----------|---------|---------|
| Web Framework | Axum | 0.8.8 |
| Database | SQLite (rusqlite) | 0.38.0 |
| Connection Pool | r2d2 | 0.8.10 |
| Async Runtime | Tokio | 1.49.0 |
| Embeddings | model2vec-rs | 0.1.4 |
| Hashing | xxhash-rust | 0.8.15 |
| Regex | regex | 1.11 |

---

## License

MIT License — Free for personal and commercial use.

---

## Support

- GitHub Issues: Report bugs and feature requests
- Documentation: This file + inline code comments
- Community: OpenClaw Discord

---

**Brain Server: Your local memory and knowledge graph, powered by open source.**
