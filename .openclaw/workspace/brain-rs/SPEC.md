# Brain-Server v0.8.0 - Knowledge Graph Extension

## Overview
Add knowledge graph capabilities to brain-server by parsing structured markdown annotations in Hugo posts. No LLM required - entities and relationships are explicitly defined in the markdown.

## Storage Strategy
SQLite with indexed columns (not JSON). Fast queries, low memory.

## New Database Schema

```sql
-- Entities extracted from markdown annotations
CREATE TABLE IF NOT EXISTS entities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    entity_type TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_entities_name ON entities(name);
CREATE INDEX idx_entities_type ON entities(entity_type);

-- Relationships between entities
CREATE TABLE IF NOT EXISTS relationships (
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
CREATE INDEX idx_rels_type ON relationships(relation_type);
CREATE UNIQUE INDEX idx_rels_unique ON relationships(from_entity_id, to_entity_id, relation_type);
```

## Storage Location

### Recommended Folder Structure

Create a dedicated knowledge folder in your workspace:

```bash
mkdir -p ~/.openclaw/workspace/knowledge/{memory,posts,notes}
```

```
knowledge/
├── memory/       # Personal memories → auto-ingest nightly
├── posts/        # Hugo blog posts → ingest on build
└── notes/        # Quick notes → ingest manually
```

### Important: Separate from OpenClaw

| Component | Process | Port | Impact |
|-----------|---------|------|--------|
| **OpenClaw** | Main | 18789 | Unaffected |
| **Brain-Server** | Separate | 8765 | Only processes when called |

Brain-server is a **separate process**. It only processes when you call the API — it doesn't scan or watch files automatically, so it won't slow down OpenClaw!

### Where to Save Files

Files can be **anywhere** on your system. Just pass the path or content to the API:

```bash
# Example: Ingest from anywhere
curl -X POST http://localhost:8765/ingest/markdown \
  --data-binary @/path/to/any/file.md

# Or pass content directly
curl -X POST http://localhost:8765/ingest/markdown \
  -d "Content here"
```

## Supported Annotation Formats

### 1. Wiki-style Links (recommended)
```markdown
Bignay is [[alternative_to::blueberry]].
It has [[has_property::anthocyanins]].
Related to: [[related_to::duhat]], [[related_to::mulberry]].
```

### 2. Relation Shorthand
```markdown
bignay > blueberry (alternative_to)
bignay > anthocyanins (has_property)
duhat > mulberry (related_to)
```

### 3. Frontmatter (for Hugo)
```yaml
---
entities:
  - name: bignay
    type: food
  - name: blueberry
    type: food
relations:
  - from: bignay
    to: blueberry
    type: alternative_to
---
```

## Supported Relation Types

| Type | Meaning | Example |
|------|---------|---------|
| `alternative_to` | X is alternative to Y | bignay > blueberry |
| `related_to` | X related to Y | duhat > mulberry |
| `has_property` | X has property Y | bignay > anthocyanins |
| `best_for` | X best for Y | duhat > blood sugar |
| `source_from` | X from Y | mango > philippines |
| `manages` | X manages Y | alice > auth_team |
| `is_a` | X is a Y | bignay > superfruit |

### POST /ingest/directory
Batch ingest all markdown files in a directory.

```
POST /ingest/directory
Body: {"path": "/path/to/folder/"}
```

Response:
```json
{
  "success": true,
  "files_processed": 253,
  "knowledge_added": 847,
  "entities_extracted": 2341,
  "relationships_mapped": 1892,
  "duplicates_skipped": 45,
  "duration_ms": 45230
}
```

### Example: Ingest Your GutMindSynergy Posts

```bash
curl -X POST http://localhost:8765/ingest/directory \
  -H "Content-Type: application/json" \
  -d '{"path": "/home/jetson/.openclaw/workspace/macbook-sites/gutmindsynergy/posts/"}'
```

## API Endpoints

### Graph Queries

```
GET /graph/entity/:name
GET /graph/relations?from=:entity
GET /graph/traverse?entity=:name&relation=:type&depth=:n
GET /graph/search?q=:query&enrich=true
```

### Ingestion

```
POST /ingest/markdown     - Ingest raw markdown file
POST /ingest/memory       - Already exists, updated to also extract graph
GET  /stats               - Updated: includes entity/relation counts
```

## Parser Logic

```rust
fn parse_annotations(content: &str) -> (Vec<Entity>, Vec<Relation>) {
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    
    // Pattern 1: [[relation::entity]]
    let re = Regex::new(r"\[\[(\w+)::([^\]]+)\]\]").unwrap();
    for cap in re.captures_iter(content) {
        let rel_type = cap.get(1).unwrap().as_str();
        let target = cap.get(2).unwrap().as_str().trim();
        
        // Extract entity names (handle comma-separated)
        for entity_name in target.split(',') {
            let name = entity_name.trim();
            if !name.is_empty() {
                entities.push(Entity { name: name.to_string(), entity_type: None });
                relations.push(Relation { 
                    from: current_topic.clone(),  // Track current heading
                    to: name.to_string(), 
                    rel_type: rel_type.to_string() 
                });
            }
        }
    }
    
    (entities, relations)
}
```

## Query Examples

### Find blueberry alternatives
```sql
SELECT e2.name, r.relation_type
FROM relationships r
JOIN entities e1 ON r.from_entity_id = e1.id
JOIN entities e2 ON r.to_entity_id = e2.id
WHERE e1.name = 'blueberry'
AND r.relation_type = 'alternative_to';
```

### Graph traversal (2 hops)
```sql
WITH RECURSIVE traverse AS (
    SELECT from_entity_id, to_entity_id, relation_type, 1 as depth
    FROM relationships
    WHERE from_entity_id = (SELECT id FROM entities WHERE name = 'bignay')
    
    UNION ALL
    
    SELECT r.from_entity_id, r.to_entity_id, r.relation_type, t.depth + 1
    FROM relationships r
    JOIN traverse t ON r.from_entity_id = t.to_entity_id
    WHERE t.depth < 2
)
SELECT * FROM traverse;
```

## Combined Search Response

```json
{
  "success": true,
  "results": [
    {
      "id": 1,
      "title": "Philippine Superfruits",
      "content": "Bignay is...",
      "similarity": 0.85
    }
  ],
  "graph_enrichment": {
    "entities_found": ["bignay", "blueberry", "duhat"],
    "relationships": [
      {"from": "bignay", "to": "blueberry", "type": "alternative_to"},
      {"from": "bignay", "to": "duhat", "type": "related_to"}
    ],
    "insights": "Based on the knowledge graph, bignay is the Philippine alternative to blueberry with similar antioxidant properties."
  }
}
```

## Storage Estimates

| Component | Estimate |
|-----------|----------|
| Entities (3 per entry) | ~1,200 × 50 bytes = 60KB |
| Relationships (2 per entry) | ~800 × 40 bytes = 32KB |
| Indexes | ~50KB |
| **Total** | ~150KB |

## Migration

```rust
fn run_migration(db: &mut Connection) -> Result<()> {
    // Existing tables... 
    
    // NEW: Entities table
    db.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            entity_type TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)",
        [],
    )?;
    
    // NEW: Relationships table
    db.execute(
        "CREATE TABLE IF NOT EXISTS relationships (
            id INTEGER PRIMARY KEY,
            from_entity_id INTEGER NOT NULL,
            to_entity_id INTEGER NOT NULL,
            relation_type TEXT NOT NULL,
            knowledge_id INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(from_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
            FOREIGN KEY(to_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
            FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE SET NULL
        )",
        [],
    )?;
    
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_rels_from ON relationships(from_entity_id)",
        [],
    )?;
    
    db.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_rels_unique 
         ON relationships(from_entity_id, to_entity_id, relation_type)",
        [],
    )?;
    
    Ok(())
}
```

## Updated Stats Endpoint

```rust
async fn stats(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // ... existing knowledge stats ...
    
    let entity_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entities", [], |r| r.get(0)
    )?;
    
    let relation_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM relationships", [], |r| r.get(0)
    )?;
    
    Json(serde_json::json!({
        "count": count,
        "embeddings": embed_count,
        "entities": entity_count,
        "relationships": relation_count,
        "model": MODEL_ID,
        "version": "0.8.0"
    }))
}
```

## Backward Compatibility

- Existing `/search` endpoint unchanged
- Existing `/ingest/memory` still works (enhanced)
- New endpoints added: `/graph/*`
- All existing data preserved

## Testing

```bash
# Test markdown parsing
curl -X POST http://localhost:8765/ingest/markdown \
  -d "Bignay is [[alternative_to::blueberry]]. It has [[has_property::anthocyanins]]."

# Query graph
curl "http://localhost:8765/graph/entity/bignay"
curl "http://localhost:8765/graph/traverse?entity=bignay&relation=related_to&depth=2"

# Combined search
curl "http://localhost:8765/search?q=blueberry+alternatives&enrich=true"

# Batch ingest directory
curl -X POST http://localhost:8765/ingest/directory \
  -H "Content-Type: application/json" \
  -d '{"path": "~/.openclaw/workspace/knowledge/memory/"}'
```

## Recommended Cron Jobs (Auto-Ingest)

### Nightly Memory Sync
```bash
# Daily at midnight - ingest personal memories
0 0 * * * curl -s -X POST http://localhost:8765/ingest/directory \
  -H "Content-Type: application/json" \
  -d '{"path": "/home/jetson/.openclaw/workspace/knowledge/memory/"}'
```

### Weekly Posts Sync
```bash
# Every Sunday at 2am - ingest blog posts
0 2 * * 0 curl -s -X POST http://localhost:8765/ingest/directory \
  -H "Content-Type: application/json" \
  -d '{"path": "/home/jetson/.openclaw/workspace/knowledge/posts/"}'
```

Note: These cron jobs can be set up in OpenClaw's gateway using the `cron` tool.
