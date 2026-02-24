# Brain Server v0.8.1

A high-performance semantic memory server with knowledge graph capabilities, optimized for ARM Cortex-A57 (Jetson Nano).

## Overview

Brain Server provides:
- **Semantic Search** - Vector embeddings using model2vec (minishlab/potion-retrieval-32M)
- **Knowledge Graph** - Entity and relationship extraction with graph traversal
- **Domain Annotations** - Automatic intelligent annotation of markdown content during ingestion
- **SQLite Storage** - Lightweight embedded database with connection pooling

---

## Quick Start

```bash
# Build (Jetson Nano optimized)
cd ~/.openclaw/workspace/brain-rs
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C codegen-units=1" cargo build --release -j 1

# Run
./target/release/brain-server

# Or with systemd
systemctl --user start brain-server
```

---

## Annotation System

### What Are Annotations?

The annotation system automatically extracts entities and relationships from markdown content during ingestion. Instead of manually tagging content with `[[relation::entity]]` syntax, the system can intelligently identify:

- **Entities**: People, places, concepts, products, etc.
- **Relationships**: How entities relate to each other

### How It Works

1. **Domain Configuration**: TOML files define entities, relations, and patterns for each knowledge domain
2. **Ingestion Pipeline**: When markdown is ingested, the annotator extracts matching entities
3. **Relationship Detection**: Patterns like "X helps Y" or "X is located in Y" create relationships
4. **Knowledge Graph**: Extracted data populates the graph for traversal queries

### Enabling Annotations

Annotations are controlled by the `ANNOTATOR_ENABLED` environment variable:

```bash
# Enable annotations (default: disabled)
ANNOTATOR_ENABLED=true ./target/release/brain-server

# Or in systemd service file
Environment=ANNOTATOR_ENABLED=true
```

### Annotation Configuration Directory

Configuration files are loaded from:

```
~/.brain-domains/
├── business.toml           # Business, AI consulting, tax optimization
├── health.toml             # Supplements, nutrition, gut health
├── technology.toml         # Programming, frameworks, tools
├── travel_pets.toml        # Travel, pets, relocation
└── philippines_expat.toml  # 13A visa, immigration, expat living
```

---

## Annotation Configuration Format

Each domain configuration file follows this structure:

```toml
[domain]
name = "Domain Name"
version = "1.0"
description = "What this domain covers"

[entities]
# Named entity groups
supplements = ["vitamin d3", "omega-3", "magnesium"]
conditions = ["inflammation", "gut issues", "thyroid problems"]
foods = ["bone broth", "sauerkraut", "green tea"]

[relations]
# Relationship types with trigger patterns
treats = ["treats", "helps", "improves", "alleviates"]
supports = ["supports", "promotes", "enhances", "boosts"]
requires = ["requires", "needs", "must have"]

[patterns]
# Regex patterns for complex matching
dosage_patterns = ["(?i)\\b[0-9,]+\\s*(IU|mg|mcg|g)\\b"]
date_patterns = ["(?i)\\b(january|february)\\s+[0-9]{1,2},?\\s+[0-9]{4}\\b"]

[aliases]
# Synonyms and abbreviations
omega3 = ["omega-3", "n-3", "fish oil", "fatty acids"]
vit_d = ["vitamin d", "vit d3", "cholecalciferol"]

[high_priority_entities]
# Always extract these, even without relationships
critical = ["vitamin d3", "omega-3", "probiotics"]

[exclusions]
# Words to ignore
exclude = ["just", "very", "really", "quite", "too"]
```

---

## API Endpoints

### Ingest Markdown with Auto-Annotations

```bash
# POST /ingest/markdown
curl -X POST http://localhost:8765/ingest/markdown \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Vitamin D3 Benefits",
    "content": "Vitamin D3 supports immune system function. It helps with inflammation and requires magnesium for absorption. Good sources include fatty fish like bangus."
  }'
```

**What gets extracted:**
- Entities: `vitamin d3`, `immune system`, `inflammation`, `magnesium`, `bangus`
- Relationships:
  - `vitamin d3` --supports--> `immune system`
  - `vitamin d3` --helps--> `inflammation`
  - `vitamin d3` --requires--> `magnesium`

### Manual Annotation Syntax

You can also use explicit annotations in markdown:

```markdown
Bignay is [[alternative_to::blueberry]].
It has [[has_property::antioxidants]].
Grows in [[located_in::southeast_asia]].
```

### Query the Knowledge Graph

```bash
# Get entity details
curl http://localhost:8765/graph/entity/vitamin_d3

# Get relationships
curl "http://localhost:8765/graph/relations?from=vitamin_d3"

# Traverse graph (depth 1-3)
curl "http://localhost:8765/graph/traverse?entity=inflammation&depth=2"
```

### Semantic Search

```bash
curl "http://localhost:8765/search?q=supplements+for+inflammation&k=5"
```

### Health & Stats

```bash
curl http://localhost:8765/health
curl http://localhost:8765/stats | jq '.'
```

---

## Domain Examples

### Health Domain (health.toml)

Extracts supplements, conditions, foods, and wellness concepts:

```toml
[entities]
supplements = ["vitamin d3", "vitamin k2", "omega-3", "magnesium", "zinc", "probiotics"]
conditions = ["inflammation", "gut issues", "leaky gut", "thyroid problems"]
gut_health = ["microbiome", "gut bacteria", "butyrate", "short chain fatty acids"]

[relations]
treats = ["treats", "helps", "improves", "alleviates"]
supports = ["supports", "promotes", "enhances", "boosts"]
requires = ["requires", "needs", "absorbed with", "best taken with"]
```

**Example extraction:**
```
"Omega-3 supports brain health and helps reduce inflammation"
→ omega-3 --supports--> brain health
→ omega-3 --helps--> inflammation
```

### Philippines Expat Domain (philippines_expat.toml)

Extracts visa types, locations, documents, and processes:

```toml
[entities]
visas = ["13a visa", "balikbayan", "srrv", "tourist visa"]
locations = ["kabankalan", "bacolod", "manila", "cebu"]
documents = ["passport", "birth certificate", "marriage certificate", "nbi clearance"]

[relations]
requires = ["requires", "needs", "must have"]
issued_by = ["issued by", "obtained from", "provided by"]
located_in = ["located in", "in", "at", "based in"]
```

**Example extraction:**
```
"13A visa requires passport and police clearance, processed in Bacolod"
→ 13a visa --requires--> passport
→ 13a visa --requires--> police clearance
→ 13a visa --processed_in--> bacolod
```

### Business Domain (business.toml)

Extracts business concepts, strategies, and financial terms:

```toml
[entities]
business_types = ["ai agency", "startup", "consulting", "llc", "s corporation"]
strategies = ["tax optimization", "pricing strategy", "growth strategy"]
financial = ["revenue", "profit margin", "cash flow", "tax deduction"]

[relations]
generates = ["generates", "produces", "creates", "yields"]
targets = ["targets", "focuses on", "serves"]
optimizes = ["optimizes", "improves", "enhances"]
```

---

## Building & Deployment

### Jetson Nano Build

```bash
# Optimized build for ARM Cortex-A57
cd ~/.openclaw/workspace/brain-rs

# Build with single core (prevents OOM on 4GB RAM)
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C codegen-units=1" \
  cargo build --release -j 1
```

### Binary Location

```
~/.openclaw/workspace/brain-rs/target/release/brain-server
```

### Systemd Service

```bash
# Install
cp brain-server ~/.openclaw/workspace/brain-server
systemctl --user enable brain-server
systemctl --user start brain-server

# Status
systemctl --user status brain-server

# Logs
journalctl --user -u brain-server -f
```

### Service File Example

`~/.config/systemd/user/brain-server.service`:

```ini
[Unit]
Description=Brain Server v0.8.0 (Rust/NEON)
After=network.target

[Service]
Type=simple
ExecStartPre=/bin/sleep 2
ExecStart=/home/jetson/.openclaw/workspace/brain-server
Environment=ANNOTATOR_ENABLED=true
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_HOST` | `127.0.0.1` | Host to bind to |
| `BIND_PORT` | `8765` | Port to listen on |
| `ANNOTATOR_ENABLED` | `false` | Enable domain annotation extraction |

### CORS Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `CORS_ORIGINS` | `http://localhost:3000,http://localhost:8080` | Allowed origins (comma-separated, use `*` for any) |
| `CORS_METHODS` | `GET,POST,PUT,DELETE,OPTIONS` | Allowed HTTP methods (comma-separated) |
| `CORS_HEADERS` | `content-type,authorization` | Allowed request headers (comma-separated) |

**Security Note:** For production, always explicitly set `CORS_ORIGINS` to your specific domains rather than using `*`.

```bash
# Production example
CORS_ORIGINS=https://example.com,https://app.example.com ./target/release/brain-server
```

---

## Database Schema

### knowledge
Main content storage with embeddings.

### entities
```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    entity_type TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### relationships
```sql
CREATE TABLE relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity_id INTEGER NOT NULL,
    to_entity_id INTEGER NOT NULL,
    relation_type TEXT NOT NULL,
    knowledge_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(from_entity_id) REFERENCES entities(id),
    FOREIGN KEY(to_entity_id) REFERENCES entities(id),
    FOREIGN KEY(knowledge_id) REFERENCES knowledge(id)
);
```

---

## Adding Custom Domains

1. Create a new TOML file in `~/.brain-domains/`:

```bash
nano ~/.brain-domains/custom.toml
```

2. Define your entities, relations, and patterns:

```toml
[domain]
name = "Custom Domain"
version = "1.0"
description = "My custom knowledge domain"

[entities]
concepts = ["concept1", "concept2", "concept3"]

[relations]
relates_to = ["relates to", "connects to", "links to"]

[exclusions]
exclude = ["just", "very"]
```

3. Restart the server to load the new domain

---

## Troubleshooting

### Annotations Not Working

1. Check `ANNOTATOR_ENABLED=true` is set
2. Verify config files exist in `~/.brain-domains/`
3. Check server logs: `journalctl --user -u brain-server -f`

### Build Fails

1. Ensure using single core: `cargo build --release -j 1`
2. Check available memory: `free -h`
3. Try without LTO: remove `-C lto=fat` from RUSTFLAGS

### Database Locked

The connection pool handles this automatically. If issues persist:
- Check pool health: `curl http://localhost:8765/health`
- Restart the service

---

## Version History

### v0.8.0 (2026-02-19)
- Knowledge graph support (entities, relationships)
- Domain-based annotation system
- Graph traversal API
- Pool health monitoring
- Prompt injection detection
- ARM Cortex-A57 optimization

---

## License

MIT
