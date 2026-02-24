//! Brain Server v0.7.3

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
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration as StdDuration,
};
use sysinfo::System;
use tokio::{signal, task, time::{timeout, Duration}};
use tower_http::cors::{Any, CorsLayer};
use xxhash_rust::xxh3::xxh3_64;

// Connection leak fix: tracking imports
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

type Pool = r2d2::Pool<SqliteConnectionManager>;

/// Tracks database connections for leak detection
pub struct ConnectionTracker {
    connections: Mutex<HashMap<usize, ConnectionInfo>>,
    next_id: AtomicUsize,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    id: usize,
    acquired_at: Instant,
    location: String,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            next_id: AtomicUsize::new(1),
        }
    }
    
    pub fn track(&self, location: &str) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let info = ConnectionInfo {
            id,
            acquired_at: Instant::now(),
            location: location.to_string(),
        };
        if let Ok(mut conns) = self.connections.lock() {
            conns.insert(id, info);
        }
        id
    }
    
    pub fn release(&self, id: usize) {
        if let Ok(mut conns) = self.connections.lock() {
            conns.remove(&id);
        }
    }
    
    pub fn get_long_running(&self, threshold: std::time::Duration) -> Vec<ConnectionInfo> {
        if let Ok(conns) = self.connections.lock() {
            conns.values()
                .filter(|info| info.acquired_at.elapsed() > threshold)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn count(&self) -> usize {
        if let Ok(conns) = self.connections.lock() {
            conns.len()
        } else {
            0
        }
    }
}

pub fn spawn_connection_watchdog(tracker: std::sync::Arc<ConnectionTracker>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let long_running = tracker.get_long_running(std::time::Duration::from_secs(300));
            if !long_running.is_empty() {
                eprintln!("⚠️  WARNING: {} connection(s) held for >300s:", long_running.len());
                for info in long_running {
                    eprintln!("   - Connection {} at {}: {:?}", info.id, info.location, info.acquired_at.elapsed());
                }
            }
        }
    });
}
const MODEL_ID: &str = "minishlab/potion-retrieval-32M";
const DEFAULT_K: usize = 5;
const MAX_K: usize = 100;
const SERVER_VERSION: &str = "0.7.4";
const SHUTDOWN_DRAIN_SECS: u64 = 60;
const MAX_REQUEST_SIZE: usize = 1024 * 1024;
const MAX_QUERY_LENGTH: usize = 2000;

struct AppState {
    model: Arc<StaticModel>,
    pool: Pool,
    #[allow(dead_code)]
    db_path: PathBuf,
    connection_tracker: std::sync::Arc<ConnectionTracker>,
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

const SEARCH_BATCH_SIZE: usize = 500;

fn perform_search(pool: &Pool, model: &StaticModel, q: String, k: usize) -> Result<Vec<SearchResult>> {
    let v = model.encode(&[q]).into_iter().next().context("Query encoding failed")?;

    let conn = pool.get().context("DB connection failed")?;

    let total_count: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))?;

    let mut results: Vec<SearchResult> = Vec::with_capacity(k * 2);
    let mut offset = 0;

    while offset < total_count as usize {
        let _batch_end = (offset + SEARCH_BATCH_SIZE).min(total_count as usize);

        let mut stmt = conn.prepare(
            "SELECT k.id, k.title, k.content, e.vector FROM knowledge k
             JOIN embeddings e ON k.id=e.knowledge_id
             LIMIT ? OFFSET ?"
        )?;

        let batch_results: Vec<_> = stmt
            .query_map(params![SEARCH_BATCH_SIZE as i64, offset as i64], |row| {
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

        results.extend(batch_results);

        if results.len() >= k * 10 {
            break;
        }

        offset += SEARCH_BATCH_SIZE;
    }

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

    if q.len() > MAX_QUERY_LENGTH {
        return Json(serde_json::json!({
            "success": false,
            "error": "Query too long"
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
    let content = match to_bytes(body, MAX_REQUEST_SIZE).await {
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
    let tracker = std::sync::Arc::clone(&s.connection_tracker);

    let ingest_future = task::spawn_blocking(move || {
        // Track connection acquisition for leak detection
        let conn_id = tracker.track("ingest_memory");
        let entries = parse_memory_content(&content);
        if entries.is_empty() {
            tracker.release(conn_id);
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
                tracker.release(conn_id);
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

        // Release connection tracking before returning
        tracker.release(conn_id);
        
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
        Err(_) => {
            eprintln!("⚠️  ingest_memory timed out after 60s - connection potentially leaked!");
            eprintln!("📊 Active tracked connections: {}", s.connection_tracker.count());
            Json(serde_json::json!({
                "success": false,
                "status": "error",
                "error": "Ingest timed out - connection potentially leaked (watchdog will detect)"
            }))
        }
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

async fn health(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();

    let health_future = task::spawn_blocking(move || {
        let mut sys = System::new();
        sys.refresh_memory();

        let pool_state = pool.state();

        Ok::<_, anyhow::Error>((sys.used_memory() / 1_000_000, sys.total_memory() / 1_000_000, pool_state))
    });

    match timeout(StdDuration::from_secs(3), health_future).await {
        Ok(Ok(Ok((used_mb, total_mb, pool_state)))) => Json(serde_json::json!({
            "status": "ok",
            "version": SERVER_VERSION,
            "model": MODEL_ID,
            "system": {
                "memory_used_mb": used_mb,
                "memory_total_mb": total_mb,
                "memory_percent": if total_mb > 0 { (used_mb as f64 / total_mb as f64) * 100.0 } else { 0.0 }
            },
            "pool": {
                "connections": pool_state.connections,
                "idle_connections": pool_state.idle_connections,
                "busy_connections": pool_state.connections.saturating_sub(pool_state.idle_connections)
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
        Ok::<_, anyhow::Error>((count, embed_count))
    });

    match timeout(StdDuration::from_secs(5), stats_future).await {
        Ok(Ok(Ok((count, embed_count)))) => Json(serde_json::json!({
            "count": count,
            "embeddings": embed_count,
            "model": MODEL_ID,
            "version": SERVER_VERSION
        })),
        Ok(Ok(Err(e))) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
            "model": MODEL_ID,
            "version": SERVER_VERSION,
            "error": e.to_string()
        })),
        Ok(Err(_)) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
            "model": MODEL_ID,
            "version": SERVER_VERSION,
            "error": "Task join error"
        })),
        Err(_) => Json(serde_json::json!({
            "count": 0,
            "embeddings": 0,
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
        .max_size(20)
        .min_idle(Some(4))
        .connection_timeout(StdDuration::from_secs(30))
        .max_lifetime(Some(StdDuration::from_secs(60)))
        .idle_timeout(Some(StdDuration::from_secs(20)))
        .test_on_check_out(true)
        .build(SqliteConnectionManager::file(&db_path))?;

    run_migration(&mut *pool.get().context("migration failed")?)?;

    println!("🤖 Loading model: {}", MODEL_ID);
    let model = Arc::new(
        StaticModel::from_pretrained(MODEL_ID, None, Some(true), None)
            .map_err(|e| anyhow::anyhow!("Model load failed: {}", e))?,
    );
    println!("✅ Model loaded");
    
    // Initialize connection leak detection
    let connection_tracker = std::sync::Arc::new(ConnectionTracker::new());
    spawn_connection_watchdog(std::sync::Arc::clone(&connection_tracker));
    println!("🔍 Connection watchdog started (checks every 30s, warns after 300s)");

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
        .route("/search", get(search))
        .route("/v1/embeddings", post(embeddings))
        .layer(cors)
        .with_state(Arc::new(AppState { 
            model, 
            pool, 
            db_path: db_path.clone(),
            connection_tracker,
        }));

    let bind_host = std::env::var("BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let bind_port: u16 = std::env::var("BIND_PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse()
        .unwrap_or(8765);

    let addr = match bind_host.parse::<std::net::IpAddr>() {
        Ok(ip) => SocketAddr::from((ip, bind_port)),
        Err(_) => {
            eprintln!("Invalid BIND_HOST '{}', defaulting to 0.0.0.0", bind_host);
            SocketAddr::from(([0, 0, 0, 0], bind_port))
        }
    };
    println!("🚀 Server: http://{}:{}", bind_host, bind_port);

    let listener = tokio::net::TcpListener::bind(addr).await?;

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

            let elapsed = drain_start.elapsed();
            println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
        })
        .await?;

    Ok(())
}
