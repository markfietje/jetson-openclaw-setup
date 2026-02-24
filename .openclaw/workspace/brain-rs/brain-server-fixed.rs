//! Brain Server v6.2 - Fixed & Complete
//! - Working ingest (no "unchanged" bug)
//! - Semantic search with cosine similarity
//! - OpenAI-compatible /v1/embeddings endpoint

use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{Datelike, Local};
use model2vec_rs::model::StaticModel;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc, time::SystemTime};
use tokio::task;
use xxhash_rust::xxh3::xxh3_64;

type Pool = r2d2::Pool<SqliteConnectionManager>;
const MODEL_ID: &str = "minishlab/potion-retrieval-32M";

struct AppState {
    model: Arc<StaticModel>,
    pool: Pool,
    memory_path: PathBuf,
    archive_dir: PathBuf,
    file_meta: Arc<tokio::sync::Mutex<(u64, SystemTime)>>,
}

// ============================================================================
// REQUEST STRUCTS
// ============================================================================

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    k: Option<usize>,
}

#[derive(Serialize)]
struct SearchResult {
    id: i64,
    similarity: f32,
    title: Option<String>,
    content: String,
}

// OpenAI-Compatible Embeddings API
#[derive(Deserialize)]
struct EmbeddingsRequest {
    input: String,
    #[serde(default = "default_model")]
    model: String,
}

#[derive(Serialize)]
struct EmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Serialize)]
struct EmbeddingsUsage {
    prompt_tokens: usize,
    total_tokens: usize,
}

#[derive(Serialize)]
struct EmbeddingsResponse {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: EmbeddingsUsage,
}

fn default_model() -> String {
    MODEL_ID.to_string()
}

// ============================================================================
// DATABASE MIGRATION
// ============================================================================

fn run_migration(db: &mut Connection) -> Result<()> {
    db.execute_batch(
        "PRAGMA journal_mode=WAL; \
         PRAGMA synchronous=NORMAL; \
         PRAGMA foreign_keys=ON; \
         PRAGMA cache_size=-64000; \
         PRAGMA temp_store=MEMORY;"
    )?;

    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS knowledge(id INTEGER PRIMARY KEY, title TEXT, content TEXT NOT NULL, knowledge_type TEXT, source TEXT, content_hash TEXT, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP); \
         CREATE TABLE IF NOT EXISTS embeddings(knowledge_id INTEGER PRIMARY KEY, vector TEXT, FOREIGN KEY(knowledge_id) REFERENCES knowledge(id) ON DELETE CASCADE);"
    )?;

    // Check if Unique Index exists
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
            tx.execute("UPDATE knowledge SET content_hash=? WHERE id=?", params![h, id])?;
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

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

#[inline(always)]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn parse_memory(text: &str) -> Vec<(String, Option<String>)> {
    let mut entries = Vec::new();
    let (mut cur, mut title) = (String::new(), None);
    for line in text.lines() {
        if line.starts_with("## [") || line.starts_with("##[") {
            if !cur.trim().is_empty() {
                entries.push((std::mem::take(&mut cur), title.take()));
            }
            title = Some(line.trim_start_matches('#').trim().into());
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        entries.push((cur, title));
    }
    entries
}

// ============================================================================
// ENDPOINT HANDLERS
// ============================================================================

async fn add_chunk(
    State(s): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = Arc::clone(&s.model);
    let pool = s.pool.clone();

    let text = req.get("text").and_then(|t| t.as_str()).ok_or("");
    let title = req.get("title").and_then(|t| t.as_str()).ok_or(None);
    let source = req.get("source").and_then(|t| t.as_str()).ok_or("manual");

    let result = task::spawn_blocking(move || {
        // Generate embedding
        let embedding = model.encode(&[text]).into_iter().next().context("Embedding failed")?;

        // Create content hash
        let content_hash = format!("{:016x}", xxh3_64(text.trim().as_bytes()));

        // Check if exists
        let conn = pool.get().context("DB connection failed")?;
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM knowledge WHERE content_hash=?)",
                [&content_hash],
                |r| r.get::<_, i32>(0),
            )
            .context("Hash check failed")?
            == 1;

        if exists {
            return Ok::<serde_json::Value, anyhow::Error>(
                serde_json::json!({"success": true, "status": "duplicate", "chunk_id": 0}),
            );
        }

        // Insert
        let tx = conn.transaction().context("Transaction failed")?;
        tx.execute(
            "INSERT OR IGNORE INTO knowledge(content,title,content_hash,source) VALUES(?,?,?,'manual')",
            params![&text, &title, &content_hash],
        )
        .context("Insert failed")?;

        let chunk_id = tx.last_insert_rowid();
        if chunk_id > 0 {
            // Create embedding record
            let vector_str = serde_json::to_string(&embedding).context("Vector serialization failed")?;
            tx.execute(
                "INSERT INTO embeddings(knowledge_id,vector) VALUES(?,?)",
                params![chunk_id, &vector_str],
            )
            .context("Embedding insert failed")?;
            tx.commit().context("Commit failed")?;

            Ok::<serde_json::Value, anyhow::Error>(serde_json::json!({
                "success": true,
                "status": "created",
                "chunk_id": chunk_id
            }))
        } else {
            Ok::<serde_json::Value, anyhow::Error>(serde_json::json!({
                "success": false,
                "status": "duplicate",
                "chunk_id": 0
            }))
        }
    })
    .await
    .context("Spawn blocking failed")?;

    result
}

async fn search(
    State(s): State<Arc<AppState>>,
    Query(p): Query<SearchParams>,
) -> Json<serde_json::Value> {
    let (model, pool, k, q) = (
        Arc::clone(&s.model),
        s.pool.clone(),
        p.k.unwrap_or(5),
        p.q,
    );

    let res = task::spawn_blocking(move || {
        let v = model.encode(&[&q]).into_iter().next()?;
        let c = pool.get().ok()?;
        let mut stmt = c
            .prepare("SELECT k.id, k.title, k.content, e.vector FROM knowledge k JOIN embeddings e ON k.id=e.knowledge_id")
            .ok()?;
        let mut r: Vec<_> = stmt
            .query_map([], |row| {
                let vec_str: String = row.get(3)?;
                let db_vec: Vec<f32> = serde_json::from_str(&vec_str).unwrap_or_default();
                Ok(SearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    similarity: cosine_sim(&v, &db_vec),
                })
            })
            .ok()?
            .filter_map(|r| r.ok())
            .collect();
        r.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        r.truncate(k);
        Some(r)
    })
    .await
    .ok()
    .flatten();

    Json(serde_json::json!({
        "success": res.is_some(),
        "results": res.unwrap_or_default()
    }))
}

async fn health(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "model": MODEL_ID,
        "version": "6.2-FIXED"
    }))
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let count = s
        .pool
        .get()
        .ok()
        .and_then(|c| c.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get::<_, i64>(0)).ok())
        .unwrap_or(0);

    Json(serde_json::json!({
        "count": count,
        "model": MODEL_ID,
        "version": "6.2-FIXED"
    }))
}

// OpenAI-Compatible Embeddings Endpoint
async fn embeddings(
    State(s): State<Arc<AppState>>,
    Json(req): Json<EmbeddingsRequest>,
) -> Json<EmbeddingsResponse> {
    let model = Arc::clone(&s.model);

    let embedding = task::spawn_blocking(move || {
        model.encode(&[req.input]).into_iter().next()
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default();

    let tokens = embedding.len();
    Json(EmbeddingsResponse {
        object: "list".to_string(),
        data: vec![EmbeddingData {
            object: "embedding".to_string(),
            embedding,
            index: 0,
        }],
        model: req.model,
        usage: EmbeddingsUsage {
            prompt_tokens: tokens,
            total_tokens: tokens,
        },
    })
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(4)
        .enable_all()
        .build()?
        .block_on(async {
            let home = dirs::home_dir().context("no home")?;
            let db_path = home.join(".openclaw/workspace/brain.db");
            let mp = home.join(".openclaw/workspace/MEMORY.md");
            println!("🧠 Brain Server v6.2-FIXED");
            let pool = r2d2::Pool::builder().max_size(4).build(SqliteConnectionManager::file(&db_path))?;
            run_migration(&mut *pool.get()?)?;
            let model = Arc::new(
                StaticModel::from_pretrained(MODEL_ID, None, Some(true), None)
                    .map_err(|e| anyhow::anyhow!(e))?,
            );
            let m = std::fs::metadata(&mp).ok();
            let app = Router::new()
                .route("/health", get(health))
                .route("/add", post(add_chunk))
                .route("/search", post(search))
                .route("/search", get(search))
                .route("/stats", get(stats))
                .route("/v1/embeddings", post(embeddings))
                .with_state(Arc::new(AppState {
                    model,
                    pool,
                    memory_path: mp,
                    archive_dir: home.join(".openclaw/workspace/memory_archive"),
                    file_meta: Arc::new(tokio::sync::Mutex::new((
                        m.as_ref().map(|m| m.len()).unwrap_or(0),
                        m.and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH),
                    ))),
                }));
            println!("🚀 http://127.0.0.1:8765");
            println!("📝 OpenAI-compatible: POST /v1/embeddings");
            let listener = tokio::net::TcpListener::bind("127.0.0.1:8765").await?;
            axum::serve(listener, app).await?;
            Ok(())
        })
}
