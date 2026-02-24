//! Brain Server v0.7.1

use anyhow::{Context, Result};
use axum::{
    body::{to_bytes, Body},
    extract::{Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use model2vec_rs::model::StaticModel;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration as StdDuration};
use sysinfo::System;
use tokio::{signal, task, time::{timeout, Duration}};
use tower_http::cors::{Any, CorsLayer};
use xxhash_rust::xxh3::xxh3_64;
use regex::Regex;

type Pool = r2d2::Pool<SqliteConnectionManager>;
const MODEL_ID: &str = "minishlab/potion-retrieval-32M";
const DEFAULT_K: usize = 5;
const MAX_K: usize = 100;
const SERVER_VERSION: &str = "0.8.0";
const SHUTDOWN_DRAIN_SECS: u64 = 10; // 10 seconds (was 600!)

struct AppState {
    model: Arc<StaticModel>,
    pool: Pool,
    db_path: PathBuf,
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    #[serde(default)]
    k: Option<usize>,
}

#[derive(Serialize)]
struct SearchResult {
    id: i64,
    #[serde(rename = "similarity")]
    score: f32,
    title: Option<String>,
    content: String,
}

#[derive(Deserialize)]
struct AddRequest {
    text: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default = "default_source")]
    source: String,
}

#[derive(Serialize)]
struct AddResponse {
    success: bool,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn default_source() -> String {
    "manual".to_string()
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct EmbeddingsRequest {
    input: String,
    #[serde(default = "default_model")]
    model: String,
}

fn default_model() -> String {
    MODEL_ID.to_string()
}

fn run_migration(db: &mut Connection) -> Result<()> {
    db.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA synchronous=NORMAL; \
         PRAGMA foreign_keys=ON; \
         PRAGMA cache_size=-64000; \
         PRAGMA temp_store=MEMORY;",
    )?;

    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS knowledge(
            id INTEGER PRIMARY KEY,
            title TEXT,
            content TEXT NOT NULL,
            knowledge_type TEXT,
            source TEXT DEFAULT 'manual',
            content_hash TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ); \
         CREATE TABLE IF NOT EXISTS embeddings(
            knowledge_id INTEGER PRIMARY KEY,
            vector TEXT,
            FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE CASCADE
        );",
    )?;

    let has_index: bool = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_knowledge_hash'",
            [],
            |r| r.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_index {
        println!("MIGRATION: Scrubbing duplicates...");

        let rows: Vec<(i64, String)> = db
            .prepare("SELECT id, content FROM knowledge WHERE content_hash IS NULL")?
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let tx = db.transaction()?;
        for (id, content) in rows {
            let h = format!("{:016x}", xxh3_64(content.trim().as_bytes()));
            tx.execute(
                "UPDATE knowledge SET content_hash=? WHERE id=?",
                params![h, id],
            )?;
        }
        tx.commit()?;

        db.execute(
            "DELETE FROM knowledge WHERE id NOT IN (SELECT MIN(id) FROM knowledge GROUP BY content_hash)",
            [],
        )?;
        db.execute(
            "CREATE UNIQUE INDEX idx_knowledge_hash ON knowledge(content_hash)",
            [],
        )?;
        println!("MIGRATION: Complete");
    }

    // ==== v0.8.0: Knowledge Graph Tables ====
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            entity_type TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);
        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);",
    )?;

    db.execute_batch(
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
        );
        CREATE INDEX IF NOT EXISTS idx_rels_from ON relationships(from_entity_id);
        CREATE INDEX IF NOT EXISTS idx_rels_to ON relationships(to_entity_id);
        CREATE INDEX IF NOT EXISTS idx_rels_type ON relationships(relation_type);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_rels_unique 
         ON relationships(from_entity_id, to_entity_id, relation_type);",
    )?;

    Ok(())
}

#[inline(always)]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

async fn add_chunk(State(s): State<Arc<AppState>>, Json(req): Json<AddRequest>) -> Json<AddResponse> {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return Json(AddResponse {
            success: false,
            status: "error".to_string(),
            chunk_id: None,
            error: Some("text cannot be empty".to_string()),
        });
    }

    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();
    let title = req.title.filter(|t| !t.is_empty());
    let source = req.source;

    let add_future = task::spawn_blocking(move || {
        let embedding = match model.encode(&[text.clone()]).into_iter().next() {
            Some(e) => e,
            None => {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some("Embedding generation failed".to_string()),
                };
            }
        };

        let content_hash = format!("{:016x}", xxh3_64(text.as_bytes()));

        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some(format!("DB connection failed: {}", e)),
                };
            }
        };

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM knowledge WHERE content_hash=? LIMIT 1",
                [&content_hash],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            == 1;

        if exists {
            return AddResponse {
                success: true,
                status: "duplicate".to_string(),
                chunk_id: Some(0),
                error: None,
            };
        }

        let tx = match conn.transaction() {
            Ok(t) => t,
            Err(e) => {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some(format!("Transaction failed: {}", e)),
                };
            }
        };

        if let Err(e) = tx.execute(
            "INSERT INTO knowledge(content, title, source, content_hash) VALUES(?, ?, ?, ?)",
            params![text, title, source, content_hash],
        ) {
            return AddResponse {
                success: false,
                status: "error".to_string(),
                chunk_id: None,
                error: Some(format!("Insert failed: {}", e)),
            };
        }

        let chunk_id = tx.last_insert_rowid();
        if chunk_id > 0 {
            let vector_str = match serde_json::to_string(&embedding) {
                Ok(v) => v,
                Err(e) => {
                    return AddResponse {
                        success: false,
                        status: "error".to_string(),
                        chunk_id: None,
                        error: Some(format!("Vector serialization failed: {}", e)),
                    };
                }
            };

            if let Err(e) = tx.execute(
                "INSERT INTO embeddings(knowledge_id, vector) VALUES(?, ?)",
                params![chunk_id, vector_str],
            ) {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some(format!("Embedding insert failed: {}", e)),
                };
            }

            if let Err(e) = tx.commit() {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some(format!("Commit failed: {}", e)),
                };
            }

            AddResponse {
                success: true,
                status: "created".to_string(),
                chunk_id: Some(chunk_id),
                error: None,
            }
        } else {
            AddResponse {
                success: false,
                status: "error".to_string(),
                chunk_id: None,
                error: Some("Failed to get chunk_id".to_string()),
            }
        }
    });

    match timeout(StdDuration::from_secs(30), add_future).await {
        Ok(Ok(resp)) => Json(resp),
        Ok(Err(_)) => Json(AddResponse {
            success: false,
            status: "error".to_string(),
            chunk_id: None,
            error: Some("Task join error".to_string()),
        }),
        Err(_) => Json(AddResponse {
            success: false,
            status: "error".to_string(),
            chunk_id: None,
            error: Some("Request timed out".to_string()),
        }),
    }
}

fn perform_search(pool: &Pool, model: &StaticModel, q: String, k: usize) -> Result<Vec<SearchResult>> {
    let v = model.encode(&[q]).into_iter().next().context("Query encoding failed")?;
    
    let conn = pool.get().context("DB connection failed")?;
    
    let mut stmt = conn
        .prepare("SELECT k.id, k.title, k.content, e.vector FROM knowledge k JOIN embeddings e ON k.id=e.knowledge_id")?;
    
    let mut results: Vec<_> = stmt
        .query_map([], |row| {
            let vec_str: String = row.get(3)?;
            let db_vec: Vec<f32> = serde_json::from_str(&vec_str).unwrap_or_default();
            Ok(SearchResult {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                score: cosine_sim(&v, &db_vec),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    Ok(results)
}

async fn search(
    State(s): State<Arc<AppState>>,
    Query(p): Query<SearchParams>,
) -> Json<serde_json::Value> {
    let q = p.q.trim().to_string();
    if q.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Query cannot be empty"
        }));
    }

    let k = p.k.unwrap_or(DEFAULT_K).min(MAX_K);
    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();

    let search_future = task::spawn_blocking(move || {
        let results = perform_search(&pool, &model, q, k);
        results
    });
    
    match timeout(StdDuration::from_secs(8), search_future).await {
        Ok(Ok(Ok(results))) => Json(serde_json::json!({
            "success": true,
            "results": results
        })),
        Ok(Ok(Err(e))) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
        Ok(Err(_)) => Json(serde_json::json!({
            "success": false,
            "error": "Search task failed"
        })),
        Err(_) => Json(serde_json::json!({
            "success": false,
            "error": "Search timed out"
        })),
    }
}

async fn ingest_memory(
    State(s): State<Arc<AppState>>,
    body: Body,
) -> Json<serde_json::Value> {
    let content = match to_bytes(body, 1024 * 1024).await {
        Ok(b) => String::from_utf8(b.to_vec()).unwrap_or_default().trim().to_string(),
        Err(_) => String::new(),
    };

    if content.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "status": "error",
            "message": "Empty content"
        }));
    }

    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();

    let ingest_future = task::spawn_blocking(move || {
        let entries = parse_memory_content(&content);
        if entries.is_empty() {
            return AddResponse {
                success: false,
                status: "error".to_string(),
                chunk_id: None,
                error: Some("No valid entries found".to_string()),
            };
        }

        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                return AddResponse {
                    success: false,
                    status: "error".to_string(),
                    chunk_id: None,
                    error: Some(format!("DB connection failed: {}", e)),
                };
            }
        };

        let mut added = 0;
        let mut duplicates = 0;

        for (text, title) in entries {
            let content_hash = format!("{:016x}", xxh3_64(text.as_bytes()));

            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM knowledge WHERE content_hash=? LIMIT 1",
                    [&content_hash],
                    |r| r.get::<_, i32>(0),
                )
                .unwrap_or(0)
                == 1;

            if exists {
                duplicates += 1;
                continue;
            }

            let embedding = match model.encode(&[text.clone()]).into_iter().next() {
                Some(e) => e,
                None => continue,
            };

            let tx = match conn.transaction() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if tx.execute(
                "INSERT INTO knowledge(content, title, source, content_hash) VALUES(?, ?, ?, ?)",
                params![text, title, "memory", content_hash],
            ).is_err() {
                continue;
            }

            let chunk_id = tx.last_insert_rowid();
            if chunk_id > 0 {
                let vector_str = match serde_json::to_string(&embedding) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if tx.execute(
                    "INSERT INTO embeddings(knowledge_id, vector) VALUES(?, ?)",
                    params![chunk_id, vector_str],
                ).is_err() {
                    continue;
                }

                if tx.commit().is_ok() {
                    added += 1;
                }
            }
        }

        AddResponse {
            success: true,
            status: "completed".to_string(),
            chunk_id: Some(added as i64),
            error: if duplicates > 0 {
                Some(format!("{} duplicates skipped", duplicates))
            } else {
                None
            },
        }
    });

    match timeout(StdDuration::from_secs(60), ingest_future).await {
        Ok(Ok(resp)) => Json(serde_json::json!({
            "success": resp.success,
            "status": resp.status,
            "added": resp.chunk_id,
            "error": resp.error
        })),
        Ok(Err(e)) => Json(serde_json::json!({
            "success": false,
            "status": "error",
            "error": e.to_string()
        })),
        Err(_) => Json(serde_json::json!({
            "success": false,
            "status": "error",
            "error": "Ingest timed out"
        })),
    }
}

fn parse_memory_content(text: &str) -> Vec<(String, Option<String>)> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut title = None;

    for line in text.lines() {
        if line.starts_with("## [") || line.starts_with("##[") {
            if !current.trim().is_empty() {
                entries.push((current.trim().to_string(), title));
            }
            current.clear();
            title = Some(line.trim_start_matches('#').trim().to_string());
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        entries.push((current.trim().to_string(), title));
    }

    entries
}

// ==== Knowledge Graph Parser (v0.8.0) ====

#[derive(Debug, Clone)]
struct ExtractedEntity {
    name: String,
    entity_type: Option<String>,
}

#[derive(Debug, Clone)]
struct ExtractedRelation {
    from_entity: String,
    to_entity: String,
    relation_type: String,
}

#[derive(Debug)]
struct ParsedGraph {
    entities: Vec<ExtractedEntity>,
    relations: Vec<ExtractedRelation>,
}

/// Parse [[relation::entity]] annotations from markdown content
/// Example: "Bignay is [[alternative_to::blueberry]] and [[has_property::anthocyanins]]."
fn parse_graph_annotations(content: &str, default_topic: &str) -> ParsedGraph {
    let mut entities: Vec<ExtractedEntity> = Vec::new();
    let mut relations: Vec<ExtractedRelation> = Vec::new();
    
    // Pattern: [[relation::entity]] or [[relation::entity1, entity2]]
    let re = Regex::new(r"\[\[(\w+)::([^\]]+)\]\]").unwrap();
    
    let mut current_topic = default_topic.to_string();
    
    // Also track headings as potential topics
    let heading_re = Regex::new(r"^#+\s+(.+)$").unwrap();
    
    for line in content.lines() {
        // Update current topic from headings
        if let Some(cap) = heading_re.captures(line) {
            if let Some(title) = cap.get(1) {
                current_topic = title.as_str().to_string();
            }
        }
        
        // Extract [[relation::entity]] patterns
        for cap in re.captures_iter(line) {
            let rel_type = cap.get(1).unwrap().as_str();
            let targets = cap.get(2).unwrap().as_str();
            
            // Handle comma-separated entities: [[related_to::duhat, mulberry]]
            for target in targets.split(',') {
                let target = target.trim();
                if target.is_empty() {
                    continue;
                }
                
                // Add target entity
                entities.push(ExtractedEntity {
                    name: target.to_string(),
                    entity_type: None,
                });
                
                // Add relationship if we have a topic
                if !current_topic.is_empty() {
                    relations.push(ExtractedRelation {
                        from_entity: current_topic.clone(),
                        to_entity: target.to_string(),
                        relation_type: rel_type.to_string(),
                    });
                }
            }
        }
    }
    
    ParsedGraph { entities, relations }
}

/// Insert entities and relationships into the database
fn insert_graph_data(
    conn: &Connection,
    entities: &[ExtractedEntity],
    relations: &[ExtractedRelation],
    knowledge_id: Option<i64>,
) -> Result<(i64, i64)> {
    let mut entities_added: i64 = 0;
    let mut relations_added: i64 = 0;
    
    // First, ensure all entities exist
    let mut entity_ids: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    
    for entity in entities {
        // Try to get existing entity ID
        let id: Option<i64> = conn
            .query_row(
                "SELECT id FROM entities WHERE name = ?1 COLLATE NOCASE",
                [&entity.name],
                |r| r.get(0),
            )
            .ok();
        
        if let Some(id) = id {
            entity_ids.insert(entity.name.to_lowercase(), id);
        } else {
            // Insert new entity
            conn.execute(
                "INSERT OR IGNORE INTO entities(name, entity_type) VALUES(?1, ?2)",
                params![entity.name, entity.entity_type],
            )?;
            let id = conn.last_insert_rowid();
            if id > 0 {
                entity_ids.insert(entity.name.to_lowercase(), id);
                entities_added += 1;
            }
        }
    }
    
    // Then insert relationships
    for rel in relations {
        let from_id = entity_ids.get(&rel.from_entity.to_lowercase());
        let to_id = entity_ids.get(&rel.to_entity.to_lowercase());
        
        if let (Some(&from), Some(&to)) = (from_id, to_id) {
            // Insert relationship (ignore if duplicate)
            let result = conn.execute(
                "INSERT OR IGNORE INTO relationships(from_entity_id, to_entity_id, relation_type, knowledge_id) VALUES(?1, ?2, ?3, ?4)",
                params![from, to, rel.relation_type, knowledge_id],
            )?;
            
            if result > 0 {
                relations_added += 1;
            }
        }
    }
    
    Ok((entities_added, relations_added))
}

async fn health(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let db_path = s.db_path.clone();
    
    let health_future = task::spawn_blocking(move || {
        let mut sys = System::new();
        sys.refresh_memory();
        
        let pool_status = pool.state();
        let pool_healthy = pool.get().is_ok();
        
        let disk_space = std::fs::metadata(&db_path)
            .ok()
            .and_then(|_| std::fs::metadata(db_path.parent().unwrap_or(&db_path)))
            .map(|m| {
                #[cfg(unix)]
                {
                    use std::fs::statvfs::statvfs;
                    statvfs("/").map(|fs| {
                        let available = fs.blocks_available() * fs.fragment_size();
                        let total = fs.blocks() * fs.fragment_size();
                        let used = total.saturating_sub(available);
                        let percent = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                        (available / 1_000_000_000, total / 1_000_000_000, percent)
                    }).unwrap_or((0, 0, 0.0))
                }
                #[cfg(not(unix))]
                { (0, 0, 0.0) }
            })
            .unwrap_or((0, 0, 0.0));
        
        Ok::<_, anyhow::Error>((sys.used_memory() / 1_000_000, sys.total_memory() / 1_000_000, pool_status, pool_healthy, disk_space))
    });

    match timeout(StdDuration::from_secs(3), health_future).await {
        Ok(Ok(Ok((used_mb, total_mb, pool_state, pool_healthy, (avail_gb, total_gb, disk_pct))))) => Json(serde_json::json!({
            "status": if pool_healthy { "ok" } else { "degraded" },
            "version": SERVER_VERSION,
            "model": MODEL_ID,
            "system": {
                "memory_used_mb": used_mb,
                "memory_total_mb": total_mb,
                "memory_percent": if total_mb > 0 { (used_mb as f64 / total_mb as f64) * 100.0 } else { 0.0 },
                "disk_available_gb": avail_gb,
                "disk_total_gb": total_gb,
                "disk_percent": disk_pct
            },
            "pool": {
                "healthy": pool_healthy,
                "max_size": pool_state.max_size,
                "size": pool_state.size,
                "idle_count": pool_state.idle_count,
                "busy_count": pool_state.size.saturating_sub(pool_state.idle_count)
            }
        })),
        _ => Json(serde_json::json!({
            "status": "error",
            "version": SERVER_VERSION,
            "error": "Health check failed"
        })),
    }
}

async fn ready(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let ready_future = task::spawn_blocking(move || {
        pool.get()
            .ok()
            .and_then(|c| c.query_row("SELECT 1", [], |_| Ok(true)).ok())
            .unwrap_or(false)
    });

    let ready = match timeout(StdDuration::from_secs(3), ready_future).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => false,
        Err(_) => false,
    };

    Json(serde_json::json!({
        "ready": ready
    }))
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let stats_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))?;
        let embed_count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))?;
        
        // v0.8.0: Knowledge graph stats
        let entity_count: i64 = conn.query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0)).unwrap_or(0);
        let relation_count: i64 = conn.query_row("SELECT COUNT(*) FROM relationships", [], |r| r.get(0)).unwrap_or(0);
        
        Ok::<_, anyhow::Error>((count, embed_count, entity_count, relation_count))
    });

    match timeout(StdDuration::from_secs(5), stats_future).await {
        Ok(Ok(Ok((count, embed_count, entity_count, relation_count)))) => Json(serde_json::json!({
            "count": count,
            "embeddings": embed_count,
            "entities": entity_count,
            "relationships": relation_count,
            "model": MODEL_ID,
            "version": SERVER_VERSION
        })),
        Ok(Ok(Err(e))) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
            "entities": 0,
            "relationships": 0,
            "model": MODEL_ID,
            "version": SERVER_VERSION,
            "error": e.to_string()
        })),
        Ok(Err(_)) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
            "entities": 0,
            "relationships": 0,
            "model": MODEL_ID,
            "version": SERVER_VERSION,
            "error": "Task join error"
        })),
        Err(_) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
            "entities": 0,
            "relationships": 0,
            "model": MODEL_ID,
            "version": SERVER_VERSION,
            "error": "Request timed out"
        })),
    }
}

async fn embeddings(
    State(s): State<Arc<AppState>>,
    Json(req): Json<EmbeddingsRequest>,
) -> Json<serde_json::Value> {
    let input = req.input.trim().to_string();
    if input.is_empty() {
        return Json(serde_json::json!({
            "error": {
                "message": "input is required",
                "type": "invalid_request_error"
            }
        }));
    }

    let model = Arc::clone(&s.model);
    let model_name = req.model;

    let encode_future = task::spawn_blocking(move || model.encode(&[input]).into_iter().next());

    match timeout(StdDuration::from_secs(10), encode_future).await {
        Ok(Ok(Some(embedding))) => {
            let tokens = embedding.len();
            Json(serde_json::json!({
                "object": "list",
                "data": [{
                    "object": "embedding",
                    "embedding": embedding,
                    "index": 0
                }],
                "model": model_name,
                "usage": {
                    "prompt_tokens": tokens,
                    "total_tokens": tokens
                }
            }))
        },
        Ok(Ok(None)) | Ok(Err(_)) | Err(_) => Json(serde_json::json!({
            "error": {
                "message": "Failed to generate embedding",
                "type": "server_error"
            }
        })),
    }
}

// ==== Knowledge Graph API Handlers (v0.8.0) ====

#[derive(Deserialize)]
struct IngestMarkdownRequest {
    content: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Serialize)]
struct IngestMarkdownResponse {
    success: bool,
    status: String,
    knowledge_added: i64,
    entities_added: i64,
    relations_added: i64,
    error: Option<String>,
}

async fn ingest_markdown(
    State(s): State<Arc<AppState>>,
    Json(req): Json<IngestMarkdownRequest>,
) -> Json<IngestMarkdownResponse> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Json(IngestMarkdownResponse {
            success: false,
            status: "error".to_string(),
            knowledge_added: 0,
            entities_added: 0,
            relations_added: 0,
            error: Some("content cannot be empty".to_string()),
        });
    }

    let title = req.title.clone();
    let pool = s.pool.clone();

    let ingest_future = task::spawn_blocking(move || {
        // First, parse and store the knowledge entry (for embedding)
        let content_hash = format!("{:016x}", xxh3_64(content.as_bytes()));
        
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                return IngestMarkdownResponse {
                    success: false,
                    status: "error".to_string(),
                    knowledge_added: 0,
                    entities_added: 0,
                    relations_added: 0,
                    error: Some(format!("DB connection failed: {}", e)),
                };
            }
        };

        // Check if already exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM knowledge WHERE content_hash=? LIMIT 1",
                [&content_hash],
                |r| r.get::<_, i32>(0),
            )
            .unwrap_or(0)
            == 1;

        let knowledge_id = if exists {
            // Get existing ID
            conn.query_row(
                "SELECT id FROM knowledge WHERE content_hash=?",
                [&content_hash],
                |r| r.get(0),
            )
            .ok()
        } else {
            // Insert new knowledge entry (we need embedding for this, skip for now)
            // For markdown ingest, we'll create a placeholder without embedding
            // or use a simpler approach - just store entities/relations
            None
        };

        // Parse graph annotations from content
        let topic = title.clone().unwrap_or_else(|| "unknown".to_string());
        let parsed = parse_graph_annotations(&content, &topic);
        
        // Insert entities and relationships
        let (entities_added, relations_added) = match insert_graph_data(
            &conn,
            &parsed.entities,
            &parsed.relations,
            knowledge_id,
        ) {
            Ok((e, r)) => (e, r),
            Err(e) => {
                return IngestMarkdownResponse {
                    success: false,
                    status: "error".to_string(),
                    knowledge_added: 0,
                    entities_added: 0,
                    relations_added: 0,
                    error: Some(format!("Graph insert failed: {}", e)),
                };
            }
        };

        IngestMarkdownResponse {
            success: true,
            status: "completed".to_string(),
            knowledge_added: if knowledge_id.is_some() { 1 } else { 0 },
            entities_added,
            relations_added,
            error: None,
        }
    });

    match timeout(StdDuration::from_secs(30), ingest_future).await {
        Ok(Ok(resp)) => Json(resp),
        Ok(Err(e)) => Json(IngestMarkdownResponse {
            success: false,
            status: "error".to_string(),
            knowledge_added: 0,
            entities_added: 0,
            relations_added: 0,
            error: Some(format!("Task error: {}", e)),
        }),
        Err(_) => Json(IngestMarkdownResponse {
            success: false,
            status: "error".to_string(),
            knowledge_added: 0,
            entities_added: 0,
            relations_added: 0,
            error: Some("Request timed out".to_string()),
        }),
    }
}

#[derive(Serialize)]
struct EntityResponse {
    id: i64,
    name: String,
    entity_type: Option<String>,
    relations: Vec<RelationInfo>,
}

#[derive(Serialize)]
struct RelationInfo {
    to_entity: String,
    relation_type: String,
    direction: String,
}

async fn get_entity(
    State(s): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let name_decoded = name.clone();

    let entity_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        
        // Get entity
        let entity: Option<(i64, String, Option<String>)> = conn
            .query_row(
                "SELECT id, name, entity_type FROM entities WHERE name = ?1 COLLATE NOCASE",
                [&name_decoded],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();

        if let Some((id, name, entity_type)) = entity {
            // Get all relations (both as source and target)
            let mut relations: Vec<RelationInfo> = Vec::new();
            
            // Relations where this entity is the source
            let mut stmt = conn.prepare(
                "SELECT e.name, r.relation_type FROM relationships r
                 JOIN entities e ON r.to_entity_id = e.id
                 WHERE r.from_entity_id = ?1"
            )?;
            for row in stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))? {
                if let Ok((to_name, rel_type)) = row {
                    relations.push(RelationInfo {
                        to_entity: to_name,
                        relation_type: rel_type,
                        direction: "out".to_string(),
                    });
                }
            }
            
            // Relations where this entity is the target
            let mut stmt = conn.prepare(
                "SELECT e.name, r.relation_type FROM relationships r
                 JOIN entities e ON r.from_entity_id = e.id
                 WHERE r.to_entity_id = ?1"
            )?;
            for row in stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))? {
                if let Ok((from_name, rel_type)) = row {
                    relations.push(RelationInfo {
                        to_entity: from_name,
                        relation_type: rel_type,
                        direction: "in".to_string(),
                    });
                }
            }

            Ok(serde_json::json!({
                "id": id,
                "name": name,
                "entity_type": entity_type,
                "relations": relations
            }))
        } else {
            Ok(serde_json::json!({
                "error": "Entity not found"
            }))
        }
    });

    match timeout(StdDuration::from_secs(5), entity_future).await {
        Ok(Ok(Json(v))) => Json(v),
        Ok(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        Err(_) => Json(serde_json::json!({"error": "Request timed out"})),
    }
}

#[derive(Deserialize)]
struct RelationsQuery {
    from: Option<String>,
    to: Option<String>,
    relation_type: Option<String>,
}

async fn get_relations(
    State(s): State<Arc<AppState>>,
    Query(query): Query<RelationsQuery>,
) -> Json<serde_json::Value> {
    let pool = s.pool.clone();

    let rels_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        
        let mut results: Vec<serde_json::Value> = Vec::new();
        
        if let Some(from) = &query.from {
            let mut stmt = conn.prepare(
                "SELECT e1.name, e2.name, r.relation_type 
                 FROM relationships r
                 JOIN entities e1 ON r.from_entity_id = e1.id
                 JOIN entities e2 ON r.to_entity_id = e2.id
                 WHERE e1.name = ?1 COLLATE NOCASE"
            )?;
            for row in stmt.query_map([from], |r| {
                Ok(serde_json::json!({
                    "from": r.get::<_, String>(0)?,
                    "to": r.get::<_, String>(1)?,
                    "relation_type": r.get::<_, String>(2)?
                }))
            })? {
                if let Ok(v) = row {
                    results.push(v);
                }
            }
        } else if let Some(to) = &query.to {
            let mut stmt = conn.prepare(
                "SELECT e1.name, e2.name, r.relation_type 
                 FROM relationships r
                 JOIN entities e1 ON r.from_entity_id = e1.id
                 JOIN entities e2 ON r.to_entity_id = e2.id
                 WHERE e2.name = ?1 COLLATE NOCASE"
            )?;
            for row in stmt.query_map([to], |r| {
                Ok(serde_json::json!({
                    "from": r.get::<_, String>(0)?,
                    "to": r.get::<_, String>(1)?,
                    "relation_type": r.get::<_, String>(2)?
                }))
            })? {
                if let Ok(v) = row {
                    results.push(v);
                }
            }
        }
        
        Ok(serde_json::json!({"relations": results}))
    });

    match timeout(StdDuration::from_secs(5), rels_future).await {
        Ok(Ok(Json(v))) => Json(v),
        Ok(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        Err(_) => Json(serde_json::json!({"error": "Request timed out"})),
    }
}

#[derive(Deserialize)]
struct TraverseQuery {
    entity: String,
    #[serde(default = "default_depth")]
    depth: usize,
    #[serde(default)]
    relation_type: Option<String>,
}

fn default_depth() -> usize { 1 }

async fn traverse_graph(
    State(s): State<Arc<AppState>>,
    Query(query): Query<TraverseQuery>,
) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let entity = query.entity.clone();
    let depth = query.depth.min(3); // Cap at 3 to prevent abuse
    let rel_type = query.relation_type.clone();

    let traverse_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        
        // Simple traversal: get all entities connected to the starting entity
        let mut results: Vec<serde_json::Value> = Vec::new();
        
        // Get direct connections (depth 1)
        let mut sql = String::from(
            "SELECT e1.name, e2.name, r.relation_type 
             FROM relationships r
             JOIN entities e1 ON r.from_entity_id = e1.id
             JOIN entities e2 ON r.to_entity_id = e2.id
             WHERE e1.name = ?1 COLLATE NOCASE"
        );
        
        if let Some(ref rt) = rel_type {
            sql.push_str(&format!(" AND r.relation_type = '{}'", rt));
        }
        
        let mut stmt = conn.prepare(&sql)?;
        for row in stmt.query_map([&entity], |r| {
            Ok(serde_json::json!({
                "from": r.get::<_, String>(0)?,
                "to": r.get::<_, String>(1)?,
                "relation_type": r.get::<_, String>(2)?,
                "depth": 1
            }))
        })? {
            if let Ok(v) = row {
                results.push(v);
            }
        }
        
        // If depth > 1, get connections from depth 1 entities
        if depth > 1 {
            for result in &results {
                if let (Some(from), Some(to)) = (
                    result.get("from").and_then(|v| v.as_str()),
                    result.get("to").and_then(|v| v.as_str()),
                ) {
                    let mut stmt = conn.prepare(
                        "SELECT e1.name, e2.name, r.relation_type 
                         FROM relationships r
                         JOIN entities e1 ON r.from_entity_id = e1.id
                         JOIN entities e2 ON r.to_entity_id = e2.id
                         WHERE e1.name = ?1 COLLATE NOCASE"
                    )?;
                    for row in stmt.query_map([to], |r| {
                        Ok(serde_json::json!({
                            "from": r.get::<_, String>(0)?,
                            "to": r.get::<_, String>(1)?,
                            "relation_type": r.get::<_, String>(2)?,
                            "depth": 2
                        }))
                    })? {
                        if let Ok(v) = row {
                            results.push(v);
                        }
                    }
                }
            }
        }
        
        Ok(serde_json::json!({
            "entity": entity,
            "depth": depth,
            "connections": results
        }))
    });

    match timeout(StdDuration::from_secs(10), traverse_future).await {
        Ok(Ok(Json(v))) => Json(v),
        Ok(Err(e)) => Json(serde_json::json!({"error": e.to_string()})),
        Err(_) => Json(serde_json::json!({"error": "Request timed out"})),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Brain Server v{}", SERVER_VERSION);

    let home = dirs::home_dir().context("no home directory")?;
    let db_path = home.join(".openclaw/workspace/brain.db");

    if let Some(p) = db_path.parent() {
        std::fs::create_dir_all(p).ok();
    }

    println!("📦 Database: {:?}", db_path);

    let pool = r2d2::Pool::builder()
        .max_size(10)
        .min_idle(Some(2))
        .connection_timeout(StdDuration::from_secs(5))
        .max_lifetime(StdDuration::from_secs(30))
        .idle_timeout(StdDuration::from_secs(10))
        .test_on_check(true)
        .build(SqliteConnectionManager::file(&db_path))?;

    run_migration(&mut *pool.get().context("migration failed")?)?;

    println!("🤖 Loading model: {}", MODEL_ID);
    let model = Arc::new(
        StaticModel::from_pretrained(MODEL_ID, None, Some(true), None)
            .map_err(|e| anyhow::anyhow!("Model load failed: {}", e))?,
    );
    println!("✅ Model loaded");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/stats", get(stats))
        .route("/add", post(add_chunk))
        .route("/ingest/memory", post(ingest_memory))
        .route("/ingest/markdown", post(ingest_markdown))
        .route("/search", get(search))
        .route("/graph/entity/:name", get(get_entity))
        .route("/graph/relations", get(get_relations))
        .route("/graph/traverse", get(traverse_graph))
        .route("/v1/embeddings", post(embeddings))
        .layer(cors)
        .with_state(Arc::new(AppState { model, pool, db_path: db_path.clone() }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8765));
    println!("🚀 Server: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Clone pool for shutdown closure
    let pool_shutdown = pool.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let ctrl_c = async {
                signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {
                    println!("\n🔔 Received SIGINT (Ctrl+C)");
                },
                _ = terminate => {
                    println!("\n🔔 Received SIGTERM");
                },
            }
            
            println!("\n🛑 Initiating graceful shutdown...");
            println!("⏳ Waiting up to {} seconds for in-flight requests to complete...", SHUTDOWN_DRAIN_SECS);
            
            // Drain period - wait for requests to complete
            let drain_start = std::time::Instant::now();
            let drain_complete = async {
                // Wait for the full drain period
                tokio::time::sleep(Duration::from_secs(SHUTDOWN_DRAIN_SECS)).await;
                false // timeout reached
            };
            
            // In a real implementation, you'd track active requests
            // For now, we wait and then shutdown
            let _ = drain_complete.await;

            // 🔧 FIX: Close the connection pool!
            println!("🔌 Closing database connection pool...");
            pool_shutdown.close();
            println!("✅ Connection pool closed");

            let elapsed = drain_start.elapsed();
            println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
        })
        .await?;

    Ok(())
}
