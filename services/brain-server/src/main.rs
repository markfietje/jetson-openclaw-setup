//! Brain Server v0.8.0

use anyhow::{Context, Result};
use axum::{
    body::{to_bytes, Body},
    extract::{Query, State, Path},
    http::{header::HeaderName, Method},
    response::Json,
    routing::{get, post},
    Router,
};
use model2vec_rs::model::StaticModel;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::{AtomicUsize, Ordering}, Mutex},
    time::{Duration as StdDuration, Instant},
};
use sysinfo::System;
use tokio::{signal, task, time::{timeout, Duration}};
use tower_http::cors::{Any, CorsLayer};
use xxhash_rust::xxh3::xxh3_64;

mod annotator;

type Pool = r2d2::Pool<SqliteConnectionManager>;

const MODEL_ID: &str = "minishlab/potion-retrieval-32M";
const DEFAULT_K: usize = 5;
const MAX_K: usize = 100;
const SERVER_VERSION: &str = "0.8.0";
const SHUTDOWN_DRAIN_SECS: u64 = 60;
const MAX_REQUEST_SIZE: usize = 1024 * 1024;
const MAX_QUERY_LENGTH: usize = 2000;
const SEARCH_BATCH_SIZE: usize = 500;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    id: usize,
    acquired_at: Instant,
    location: String,
}

pub struct ConnectionTracker {
    connections: Mutex<HashMap<usize, ConnectionInfo>>,
    next_id: AtomicUsize,
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

#[allow(dead_code)]
struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: StdDuration,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests: 100,
            window: StdDuration::from_secs(60),
        }
    }

    #[allow(dead_code)]
    fn is_allowed(&self, ip: &str) -> bool {
        let now = Instant::now();
        if let Ok(mut requests) = self.requests.lock() {
            let entry = requests.entry(ip.to_string()).or_insert_with(Vec::new);
            entry.retain(|t| *t > now - self.window);
            if entry.len() >= self.max_requests {
                return false;
            }
            entry.push(now);
            true
        } else {
            true
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
                eprintln!("⚠️ WARNING: {} connection(s) held for >300s:", long_running.len());
                for info in long_running {
                    eprintln!(" - Connection {} at {}: {:?}", info.id, info.location, info.acquired_at.elapsed());
                }
            }
        }
    });
}

struct AppState {
    model: Arc<StaticModel>,
    pool: Pool,
    #[allow(dead_code)]
    db_path: PathBuf,
    connection_tracker: std::sync::Arc<ConnectionTracker>,
    #[allow(dead_code)]
    rate_limiter: Arc<RateLimiter>,
    annotator: annotator::Annotator,
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
#[serde(untagged)]
enum EmbeddingsInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Deserialize)]
struct EmbeddingsRequest {
    #[serde(deserialize_with = "deserialize_input")]
    input: EmbeddingsInput,
    #[serde(default = "default_model")]
    model: String,
}

fn deserialize_input<'de, D>(deserializer: D) -> Result<EmbeddingsInput, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawInput {
        Single(String),
        Batch(Vec<String>),
    }

    let raw = RawInput::deserialize(deserializer)?;
    Ok(match raw {
        RawInput::Single(s) => EmbeddingsInput::Single(s),
        RawInput::Batch(v) => EmbeddingsInput::Batch(v),
    })
}

fn default_model() -> String {
    MODEL_ID.to_string()
}

#[derive(Deserialize)]
struct MarkdownPayload {
    content: String,
    title: Option<String>,
}

#[derive(Serialize)]
struct IngestResponse {
    success: bool,
    id: i64,
}

#[derive(Deserialize)]
struct RelationsQuery {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Deserialize)]
struct TraverseQuery {
    start: Option<String>,
    max_depth: Option<u8>,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(&'static str),
    NotFound(&'static str),
    Internal(String),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, msg) = match self {
            AppError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
            AppError::NotFound(s) => (StatusCode::NOT_FOUND, s),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
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
         );
         CREATE TABLE IF NOT EXISTS embeddings(
            knowledge_id INTEGER PRIMARY KEY,
            vector TEXT,
            FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE CASCADE
         );",
    )?;

    // Check if deduplication migration is needed
    let has_index: bool = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_knowledge_hash'",
            [],
            |r| r.get::<_, i32>(0),
        )
        .unwrap_or(0) > 0;

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

    // v0.8.0 Knowledge Graph migration
    db.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            entity_type TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
         )",
        [],
    )?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)", [])?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)", [])?;

    db.execute(
        "CREATE TABLE IF NOT EXISTS relationships (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
    db.execute("CREATE INDEX IF NOT EXISTS idx_rels_from ON relationships(from_entity_id)", [])?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_rels_to ON relationships(to_entity_id)", [])?;
    db.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_rels_unique ON relationships(from_entity_id, to_entity_id, relation_type)",
        [],
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

    if contains_suspicious_pattern(&text) {
        return Json(AddResponse {
            success: false,
            status: "error".to_string(),
            chunk_id: None,
            error: Some("Input contains suspicious patterns".to_string()),
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
            .unwrap_or(0) == 1;

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
    let total_count: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))?;

    let mut results: Vec<SearchResult> = Vec::with_capacity(k * 2);
    let mut offset = 0;

    while offset < total_count as usize {
        let mut stmt = conn.prepare(
            "SELECT k.id, k.title, k.content, e.vector FROM knowledge k JOIN embeddings e ON k.id=e.knowledge_id LIMIT ? OFFSET ?"
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
        return Json(serde_json::json!({ "success": false, "error": "Query cannot be empty" }));
    }
    if q.len() > MAX_QUERY_LENGTH {
        return Json(serde_json::json!({ "success": false, "error": "Query too long" }));
    }
    if contains_suspicious_pattern(&q) {
        return Json(serde_json::json!({ "success": false, "error": "Input contains suspicious patterns" }));
    }

    let k = p.k.unwrap_or(DEFAULT_K).min(MAX_K);
    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();

    let search_future = task::spawn_blocking(move || {
        let results = perform_search(&pool, &model, q, k);
        results
    });

    match timeout(StdDuration::from_secs(8), search_future).await {
        Ok(Ok(Ok(results))) => Json(serde_json::json!({ "results": results })),
        Ok(Ok(Err(e))) => Json(serde_json::json!({ "error": e.to_string() })),
        Ok(Err(_)) => Json(serde_json::json!({ "error": "Search task failed" })),
        Err(_) => Json(serde_json::json!({ "error": "Search timed out" })),
    }
}

async fn ingest_memory(State(s): State<Arc<AppState>>, body: Body) -> Json<serde_json::Value> {
    let content = match to_bytes(body, MAX_REQUEST_SIZE).await {
        Ok(b) => String::from_utf8(b.to_vec()).unwrap_or_default().trim().to_string(),
        Err(_) => String::new(),
    };

    if content.is_empty() {
        return Json(serde_json::json!({ "success": false, "status": "error", "message": "Empty content" }));
    }

    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();
    let tracker = std::sync::Arc::clone(&s.connection_tracker);

    let ingest_future = task::spawn_blocking(move || {
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
                .unwrap_or(0) == 1;

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

            if tx
                .execute(
                    "INSERT INTO knowledge(content, title, source, content_hash) VALUES(?, ?, ?, ?)",
                    params![text, title, "memory", content_hash],
                )
                .is_err()
            {
                continue;
            }

            let chunk_id = tx.last_insert_rowid();
            if chunk_id > 0 {
                let vector_str = match serde_json::to_string(&embedding) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if tx
                    .execute(
                        "INSERT INTO embeddings(knowledge_id, vector) VALUES(?, ?)",
                        params![chunk_id, vector_str],
                    )
                    .is_err()
                {
                    continue;
                }

                if tx.commit().is_ok() {
                    added += 1;
                }
            }
        }

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
        Ok(Ok(resp)) => {
            let status = if resp.chunk_id == Some(0) {
                "unchanged"
            } else {
                "success"
            };
            Json(serde_json::json!({
                "status": status,
                "entry_id": resp.chunk_id.unwrap_or(0),
                "similarity_score": 1.0
            }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        Err(_) => {
            eprintln!("⚠️ ingest_memory timed out after 60s - connection potentially leaked!");
            eprintln!("📊 Active tracked connections: {}", s.connection_tracker.count());
            Json(serde_json::json!({ "status": "error", "error": "Ingest timed out" }))
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
        _ => Json(serde_json::json!({ "status": "error", "version": SERVER_VERSION, "error": "Health check failed" })),
    }
}

async fn ready(State(s): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    let pool = s.pool.clone();
    let ready_future = task::spawn_blocking(move || {
        pool.get()
            .ok()
            .and_then(|c| c.query_row("SELECT 1", [], |_| Ok(true)).ok())
            .unwrap_or(false)
    });

    match timeout(StdDuration::from_secs(3), ready_future).await {
        Ok(Ok(r)) => {
            if r {
                "OK"
            } else {
                "NOT_READY"
            }
        }
        _ => "NOT_READY",
    }
}

async fn version() -> impl axum::response::IntoResponse {
    SERVER_VERSION
}

async fn health_db(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let db_path = s.db_path.clone();

    let db_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        let metadata = std::fs::metadata(&db_path).ok();
        let db_size = metadata.map(|m| m.len()).unwrap_or(0);
        let last_write: Option<String> = conn
            .query_row("SELECT MAX(created_at) FROM knowledge", [], |r| r.get(0))
            .ok();
        let pool_state = pool.state();
        Ok::<_, anyhow::Error>((db_size, last_write, pool_state))
    });

    match timeout(StdDuration::from_secs(3), db_future).await {
        Ok(Ok(Ok((db_size, last_write, pool_state)))) => Json(serde_json::json!({
            "status": "healthy",
            "database_size_bytes": db_size,
            "database_size_mb": db_size as f64 / 1_000_000.0,
            "last_write": last_write,
            "connection_pool": {
                "active": pool_state.connections.saturating_sub(pool_state.idle_connections),
                "idle": pool_state.idle_connections,
                "max": 20
            }
        })),
        _ => Json(serde_json::json!({ "status": "error", "error": "Database health check failed" })),
    }
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pool = s.pool.clone();
    let stats_future = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))?;
        let embed_count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))?;
        let entities: i64 = conn.query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0)).unwrap_or(0);
        let relationships: i64 = conn.query_row("SELECT COUNT(*) FROM relationships", [], |r| r.get(0)).unwrap_or(0);
        Ok::<_, anyhow::Error>((count, embed_count, entities, relationships))
    });

    match timeout(StdDuration::from_secs(10), stats_future).await {
        Ok(Ok(Ok((count, embed_count, entities, relationships)))) => Json(serde_json::json!({
            "count": count,
            "embeddings": embed_count,
            "entities": entities,
            "relationships": relationships,
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
    let inputs = match &req.input {
        EmbeddingsInput::Single(s) if s.trim().is_empty() => {
            return Json(serde_json::json!({
                "error": { "message": "input is required", "type": "invalid_request_error" }
            }));
        }
        EmbeddingsInput::Single(s) => vec![s.trim().to_string()],
        EmbeddingsInput::Batch(v) if v.is_empty() => {
            return Json(serde_json::json!({
                "error": { "message": "input is required", "type": "invalid_request_error" }
            }));
        }
        EmbeddingsInput::Batch(v) => v
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    };

    if inputs.is_empty() {
        return Json(serde_json::json!({
            "error": { "message": "input is required", "type": "invalid_request_error" }
        }));
    }

    let model = Arc::clone(&s.model);
    let model_name = req.model;

    let encode_future = task::spawn_blocking(move || model.encode(&inputs));

    match timeout(StdDuration::from_secs(30), encode_future).await {
        Ok(Ok(embeddings)) => {
            let total_tokens: usize = embeddings.iter().map(|e| e.len()).sum();
            let data: Vec<_> = embeddings
                .into_iter()
                .enumerate()
                .map(|(i, emb)| serde_json::json!({ "object": "embedding", "embedding": emb, "index": i }))
                .collect();

            Json(serde_json::json!({
                "object": "list",
                "data": data,
                "model": model_name,
                "usage": { "prompt_tokens": total_tokens, "total_tokens": total_tokens }
            }))
        }
        _ => Json(serde_json::json!({
            "error": { "message": "Failed to generate embedding", "type": "server_error" }
        })),
    }
}

// === v0.8.0 KNOWLEDGE GRAPH FUNCTIONS ===

fn parse_annotations(content: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 4 <= len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i + 2;
            let mut mid = start;
            let mut found = false;

            while mid < len && mid - start < 50 {
                if bytes[mid] == b':' && mid + 1 < len && bytes[mid + 1] == b':' {
                    found = true;
                    break;
                }
                if bytes[mid] == b']' {
                    break;
                }
                mid += 1;
            }

            if found {
                let mut end = mid + 2;
                while end < len && end - start < 100 {
                    if bytes[end] == b']' && end + 1 < len && bytes[end + 1] == b']' {
                        break;
                    }
                    end += 1;
                }

                if end + 1 < len {
                    let relation = String::from_utf8_lossy(&bytes[start..mid]).trim().to_string();
                    let entity = String::from_utf8_lossy(&bytes[mid + 2..end]).trim().to_string();

                    if !relation.is_empty()
                        && !entity.is_empty()
                        && relation.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                        && entity.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    {
                        results.push((relation, entity));
                    }
                    i = end + 2;
                    continue;
                }
            }
        }
        i += 1;
    }

    results
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn contains_suspicious_pattern(input: &str) -> bool {
    let suspicious = [
        "ignore previous",
        "system:",
        "you are now",
        "### instruction",
        "### system",
        "def ",
        "import ",
        "exec(",
        "eval(",
    ];
    let lower = input.to_lowercase();
    suspicious.iter().any(|p| lower.contains(p))
}

fn build_cors_layer() -> CorsLayer {
    let cors_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:8080".to_string());

    let methods: Vec<Method> = std::env::var("CORS_METHODS")
        .unwrap_or_else(|_| "GET,POST,PUT,DELETE,OPTIONS".to_string())
        .split(',')
        .filter_map(|m| m.trim().parse().ok())
        .collect();

    let headers: Vec<HeaderName> = std::env::var("CORS_HEADERS")
        .unwrap_or_else(|_| "content-type,authorization".to_string())
        .split(',')
        .filter_map(|h| h.trim().parse().ok())
        .collect();

    let mut cors = CorsLayer::new();

    if cors_origins.trim() == "*" {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<axum::http::HeaderValue> = cors_origins
            .split(',')
            .filter_map(|origin| {
                let trimmed = origin.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse().ok()
                }
            })
            .collect();
        
        if origins.is_empty() {
            cors = cors.allow_origin(Any);
        } else {
            cors = cors.allow_origin(origins);
        }
    }

    if methods.is_empty() {
        cors = cors.allow_methods(Any);
    } else {
        cors = cors.allow_methods(methods);
    }

    if headers.is_empty() {
        cors = cors.allow_headers(Any);
    } else {
        cors = cors.allow_headers(headers);
    }

    cors
        .allow_credentials(false)
        .max_age(std::time::Duration::from_secs(3600))
}

async fn ingest_markdown(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MarkdownPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let content = payload.content.clone();
    let title = payload.title.unwrap_or_default();

    if content.len() > 1_000_000 {
        return Err(AppError::BadRequest("Content too large (max 1MB)"));
    }
    if title.len() > 500 {
        return Err(AppError::BadRequest("Title too long (max 500 chars)"));
    }
    if title.is_empty() {
        return Err(AppError::BadRequest("Title is required"));
    }
    if contains_suspicious_pattern(&content) || contains_suspicious_pattern(&title) {
        return Err(AppError::BadRequest("Input contains suspicious patterns"));
    }

    let escaped_title = html_escape(&title);

    // Parse manual [[relation::entity]] annotations
    let manual_annotations = parse_annotations(&content);

    // Automatic entity/relationship extraction
    let auto_annotations = state.annotator.annotate(&content, &title);

    // Combine manual and automatic annotations
    let mut all_annotations = Vec::new();
    for (rel, ent) in manual_annotations {
        all_annotations.push((rel, ent));
    }
    for annotation in auto_annotations {
        all_annotations.push((annotation.relation, annotation.entity));
    }

    let pool = state.pool.clone();
    let model = Arc::clone(&state.model);

    // Generate embedding for the content
    let content_for_embedding = content.clone();
    let embedding = task::spawn_blocking(move || {
        model.encode(&[content_for_embedding]).into_iter().next()
    })
    .await
    .map_err(|_| AppError::Internal("Embedding task failed".into()))?;

    let embedding_json = match embedding {
        Some(vec) => serde_json::to_string(&vec).map_err(|e| AppError::Internal(e.to_string()))?,
        None => return Err(AppError::Internal("Failed to generate embedding".into())),
    };

    let knowledge_id = task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;
        let tx = conn.transaction().map_err(|e| AppError::Internal(e.to_string()))?;

        let content_hash = format!("{:016x}", xxh3_64(content.as_bytes()));
        tx.execute(
            "INSERT OR REPLACE INTO knowledge (title, content, source, content_hash) VALUES (?1, ?2, 'markdown', ?3)",
            params![escaped_title, content, content_hash],
        ).map_err(|e| AppError::Internal(e.to_string()))?;

        let k_id = tx.last_insert_rowid();

        // Insert embedding
        tx.execute(
            "INSERT OR REPLACE INTO embeddings (knowledge_id, vector) VALUES (?1, ?2)",
            params![k_id, embedding_json],
        ).map_err(|e| AppError::Internal(e.to_string()))?;

        let from_entity = escaped_title.to_lowercase();
        tx.execute("INSERT OR IGNORE INTO entities (name) VALUES (?1)", params![from_entity])
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let from_id: i64 = tx
            .query_row("SELECT id FROM entities WHERE name = ?1", params![from_entity], |r| r.get(0))
            .map_err(|e| AppError::Internal(e.to_string()))?;

        for (relation_type, to_entity) in all_annotations {
            let to_lower = to_entity.to_lowercase();
            tx.execute("INSERT OR IGNORE INTO entities (name) VALUES (?1)", params![to_lower])
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let to_id: i64 = tx
                .query_row("SELECT id FROM entities WHERE name = ?1", params![to_lower], |r| r.get(0))
                .map_err(|e| AppError::Internal(e.to_string()))?;

            tx.execute(
                "INSERT OR IGNORE INTO relationships (from_entity_id, to_entity_id, relation_type, knowledge_id) VALUES (?1, ?2, ?3, ?4)",
                params![from_id, to_id, relation_type, k_id],
            ).map_err(|e| AppError::Internal(e.to_string()))?;
        }

        tx.commit().map_err(|e| AppError::Internal(e.to_string()))?;
        Ok::<_, AppError>(k_id)
    })
    .await
    .map_err(|_| AppError::Internal("Task join error".into()))??;

    Ok(Json(serde_json::json!({
        "success": true,
        "id": knowledge_id
    })))
}

async fn get_entity(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if name.len() > 100 || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest("Invalid entity name"));
    }

    let pool = state.pool.clone();
    let name_lower = name.to_lowercase();

    let result = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

        let entity = conn
            .query_row(
                "SELECT id, name, entity_type FROM entities WHERE name = ?1",
                params![name_lower],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)),
            )
            .ok();

        let Some((id, name, etype)) = entity else {
            return Ok(serde_json::json!({"error": "Entity not found"}));
        };

        let mut stmt = conn.prepare(
            "SELECT e.name, r.relation_type, CASE WHEN r.from_entity_id = ?1 THEN 'out' ELSE 'in' END as dir
             FROM relationships r
             JOIN entities e ON (r.to_entity_id = e.id OR r.from_entity_id = e.id)
             WHERE r.from_entity_id = ?1 OR r.to_entity_id = ?1"
        ).map_err(|e| AppError::Internal(e.to_string()))?;

        let relations: Vec<_> = stmt
            .query_map(params![id], |r| {
                Ok(serde_json::json!({
                    "to_entity": r.get::<_, String>(0)?,
                    "relation_type": r.get::<_, String>(1)?,
                    "direction": r.get::<_, String>(2)?
                }))
            })
            .map_err(|e| AppError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(serde_json::json!({
            "name": name,
            "type": etype.unwrap_or_else(|| "concept".to_string()),
            "relations": relations
        }))
    })
    .await
    .map_err(|_| AppError::Internal("Task join error".into()))??;

    Ok(Json(result))
}

async fn get_relations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RelationsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (param, is_from) = match (&params.from, &params.to) {
        (Some(f), None) if !f.is_empty() => (f.clone(), true),
        (None, Some(t)) if !t.is_empty() => (t.clone(), false),
        _ => return Err(AppError::BadRequest("Must specify 'from' or 'to'")),
    };

    let pool = state.pool.clone();
    let param_lower = param.to_lowercase();

    let result = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

        let query = if is_from {
            "SELECT e.name, r.relation_type FROM relationships r
             JOIN entities e ON r.to_entity_id = e.id
             WHERE r.from_entity_id = (SELECT id FROM entities WHERE name = ?1)"
        } else {
            "SELECT e.name, r.relation_type FROM relationships r
             JOIN entities e ON r.from_entity_id = e.id
             WHERE r.to_entity_id = (SELECT id FROM entities WHERE name = ?1)"
        };

        let mut stmt = conn.prepare(query).map_err(|e| AppError::Internal(e.to_string()))?;
        let direction = if is_from { "out" } else { "in" };

        let results: Vec<_> = stmt
            .query_map(params![param_lower], |r| {
                Ok(serde_json::json!({
                    "entity": r.get::<_, String>(0)?,
                    "relation": r.get::<_, String>(1)?,
                    "direction": direction
                }))
            })
            .map_err(|e| AppError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(serde_json::json!({ "relations": results }))
    })
    .await
    .map_err(|_| AppError::Internal("Task join error".into()))??;

    Ok(Json(result))
}

async fn traverse_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TraverseQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let entity = params.start.unwrap_or_default();
    let depth = params.max_depth.unwrap_or(2).min(3);

    if entity.is_empty() {
        return Err(AppError::BadRequest("Entity is required"));
    }
    if entity.len() > 100 || !entity.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest("Invalid entity name"));
    }

    let pool = state.pool.clone();
    let entity_lower = entity.to_lowercase();

    let result = task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::Internal(e.to_string()))?;

        let entity_id: Option<i64> = conn
            .query_row("SELECT id FROM entities WHERE name = ?1", params![entity_lower], |r| r.get(0))
            .ok();

        let Some(eid) = entity_id else {
            return Ok(serde_json::json!({ "traversal": [] }));
        };

        let query = format!(
            "WITH RECURSIVE traversal(from_id, to_id, depth, path) AS (
                SELECT from_entity_id, to_entity_id, 1, CAST(from_entity_id AS TEXT)
                FROM relationships
                WHERE from_entity_id = {eid}
                UNION ALL
                SELECT r.from_entity_id, r.to_entity_id, t.depth + 1, t.path || '->' || CAST(r.from_entity_id AS TEXT)
                FROM relationships r
                JOIN traversal t ON r.from_entity_id = t.to_id
                WHERE t.depth < {depth}
            )
            SELECT DISTINCT e.name, t.depth, t.path
            FROM traversal t
            JOIN entities e ON t.to_id = e.id"
        );

        let mut stmt = conn.prepare(&query).map_err(|e| AppError::Internal(e.to_string()))?;

        let results: Vec<_> = stmt
            .query_map([], |r| {
                Ok(serde_json::json!({
                    "entity": r.get::<_, String>(0)?,
                    "depth": r.get::<_, i64>(1)?,
                    "path": r.get::<_, String>(2)?
                }))
            })
            .map_err(|e| AppError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(serde_json::json!({ "traversal": results }))
    })
    .await
    .map_err(|_| AppError::Internal("Task join error".into()))??;

    Ok(Json(result))
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
        .min_idle(Some(2))
        .connection_timeout(StdDuration::from_secs(30))
        .max_lifetime(Some(StdDuration::from_secs(300)))
        .idle_timeout(Some(StdDuration::from_secs(60)))
        .test_on_check_out(false)
        .build(SqliteConnectionManager::file(&db_path))?;

    run_migration(&mut *pool.get().context("migration failed")?)?;
    println!("✅ Migration complete");

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

    // Spawn pool health check to prevent connection timeouts
    let pool_for_health = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Ok(conn) = pool_for_health.get() {
                let _ = conn.query_row("SELECT 1", [], |_| Ok(()));
                println!("💓 Pool health check: OK");
            }
        }
    });
    println!("💓 Pool health check started (pings DB every 30s)");

    // Initialize rate limiter
    let rate_limiter = Arc::new(RateLimiter::new());
    println!("⚡ Rate limiter initialized (100 req/min per IP)");

    // Initialize annotator
    let config_dir = dirs::home_dir()
        .unwrap()
        .join(".openclaw/workspace/.brain-domains");
    let annotator = annotator::Annotator::new(config_dir, true).unwrap_or_else(|e| {
        eprintln!("⚠️ Failed to initialize annotator: {}", e);
        eprintln!("📝 Continuing without annotation features");
        annotator::Annotator::disabled()
    });
    println!("🧠 Annotator initialized ({} domains)", annotator.domain_count());

    let cors = build_cors_layer();

    let app = Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/ready", get(ready))
        .route("/stats", get(stats))
        .route("/version", get(version))
        .route("/add", post(add_chunk))
        .route("/ingest/memory", post(ingest_memory))
        .route("/search", get(search))
        .route("/v1/embeddings", post(embeddings))
        .route("/ingest/markdown", post(ingest_markdown))
        .route("/graph/entity/{name}", get(get_entity))
        .route("/graph/relations", get(get_relations))
        .route("/graph/traverse", get(traverse_graph))
        .layer(cors)
        .with_state(Arc::new(AppState {
            model,
            pool,
            db_path: db_path.clone(),
            connection_tracker,
            rate_limiter,
            annotator,
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
                }
                _ = terminate => {
                    println!("\n🔔 Received SIGTERM");
                }
            }

            println!("\n🛑 Initiating graceful shutdown...");
            println!("⏳ Waiting up to {} seconds for in-flight requests to complete...", SHUTDOWN_DRAIN_SECS);

            let drain_start = std::time::Instant::now();
            let drain_complete = async {
                tokio::time::sleep(Duration::from_secs(SHUTDOWN_DRAIN_SECS)).await;
                false
            };

            let _ = drain_complete.await;
            let elapsed = drain_start.elapsed();
            println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
        })
        .await?;

    Ok(())
}
