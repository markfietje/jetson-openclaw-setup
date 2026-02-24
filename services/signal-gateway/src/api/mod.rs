//! Signal Gateway API Layer - OpenClaw Compatible
//!
//! Implements signal-cli REST API compatible endpoints:
//! - GET  /v1/health - Health check
//! - GET  /v1/about - About info
//! - GET  /api/v1/accounts - List accounts (OpenClaw probe)
//! - GET  /api/v1/check - Health check
//! - POST /v2/send - Send message
//! - POST /api/v1/rpc - JSON-RPC API
//! - GET  /v1/receive/{number} - WebSocket for messages
//! - GET  /api/v1/events - SSE stream for messages

use crate::state::AppState;
use axum::{
    extract::{Path, State as AxumState},
    response::{IntoResponse, Json, sse::{Event, Sse}},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and info
        .route("/v1/health", get(health_check))
        .route("/v1/about", get(about_info))
        .route("/api/v1/check", get(health_check))

        // Account management (OpenClaw probes this)
        .route("/api/v1/accounts", get(list_accounts))
        .route("/v1/accounts/{number}", get(get_account))

        // Messaging
        .route("/v2/send", post(send_message_v2))
        .route("/api/v1/rpc", post(json_rpc))

        // Cache management (for phone -> UUID mappings)
        .route("/v1/cache/seed", post(seed_cache))

        // Message streams
        .route("/v1/receive/{number}", get(receive_messages))
        .route("/api/v1/events", get(events_stream))

        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Value,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    number: Option<String>,
    recipients: Vec<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct CacheSeedRequest {
    phone: String,
    uuid: String,
}

// ==================== Health & Info ====================

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn about_info(AxumState(state): AxumState<AppState>) -> impl IntoResponse {
    let account = state.signal.get_profile().await.ok().flatten()
        .unwrap_or_else(|| "unknown".to_string());

    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "account": account
    }))
}

// ==================== Account Management ====================

/// List all registered accounts - signal-cli REST API compatible
/// This is what OpenClaw probes!
async fn list_accounts(AxumState(state): AxumState<AppState>) -> impl IntoResponse {
    let account = state.signal.get_profile().await.ok().flatten()
        .unwrap_or_else(|| "unknown".to_string());
    
    // This format matches signal-cli REST API
    Json(json!([
        {
            "address": {
                "number": account,
                "uuid": null
            },
            "enabled": true,
            "device_name": "openclaw-gateway"
        }
    ]))
}

async fn get_account(AxumState(state): AxumState<AppState>) -> impl IntoResponse {
    let account = state.signal.get_profile().await.ok().flatten()
        .unwrap_or_else(|| "unknown".to_string());
    
    Json(json!({
        "address": {
            "number": account,
            "uuid": null
        },
        "enabled": true,
        "device_name": "openclaw-gateway"
    }))
}

// ==================== Messaging ====================

/// Seed the phone -> UUID cache
/// This allows sending to phone numbers by pre-mapping them to UUIDs
async fn seed_cache(
    AxumState(state): AxumState<AppState>,
    Json(body): Json<CacheSeedRequest>,
) -> impl IntoResponse {
    let cache = state.signal.get_recipient_cache();

    // Validate inputs
    if body.phone.is_empty() || body.uuid.is_empty() {
        return Json(json!({
            "error": "Both phone and uuid are required"
        }));
    }

    // Insert into cache
    cache.insert(body.phone.clone(), body.uuid.clone());

    tracing::info!("Cached: {} -> {}", body.phone, body.uuid);

    Json(json!({
        "status": "ok",
        "phone": body.phone,
        "uuid": body.uuid,
        "message": "Cached successfully"
    }))
}

/// Send message - v2 API (OpenClaw uses this)
/// IMPORTANT: Recipient should be UUID (ACI) format, NOT phone number!
/// Signal uses UUIDs internally. Phone numbers must be resolved to UUIDs first.
async fn send_message_v2(
    AxumState(state): AxumState<AppState>,
    Json(body): Json<SendMessageRequest>,
) -> impl IntoResponse {
    // Get the first recipient
    let recipient = body.recipients.first()
        .map(|r| r.as_str())
        .unwrap_or("");

    // Handle UUID format (u:uuid) or direct UUID
    let recipient = if recipient.starts_with("u:") {
        // Extract UUID
        &recipient[2..]
    } else {
        recipient
    };

    // Log the attempt
    tracing::info!("Sending message to: {}", recipient);

    match state.signal.send_message(recipient, &body.message).await {
        Ok(_id) => Json(json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "recipient": recipient,
            "message": "Sent successfully"
        })),
        Err(e) => {
            tracing::error!("Send failed: {}", e);
            Json(json!({
                "error": e.to_string(),
                "hint": "Recipient must be a UUID (ACI), not a phone number. Use the UUID from incoming messages."
            }))
        }
    }
}

// ==================== JSON-RPC ====================

async fn json_rpc(
    AxumState(state): AxumState<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let signal = state.signal.clone();
    
    let result = handle_rpc_method(&signal, &req.method, &req.params).await;

    match result {
        Ok(value) => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id: req.id,
        }),
        Err(error_msg) => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError { 
                code: -32000, 
                message: error_msg 
            }),
            id: req.id,
        }),
    }
}

async fn handle_rpc_method(signal: &crate::signal::SignalHandle, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "sendMessage" | "send" => {
            let recipient = params["recipient"].as_str()
                .ok_or("missing recipient")?;
            let message = params["message"].as_str()
                .ok_or("missing message")?;
            
            signal.send_message(recipient, message).await
                .map(|id| json!({"timestamp": chrono::Utc::now().timestamp_millis(), "messageId": id}))
                .map_err(|e| e.to_string())
        }
        "getAccountNumber" | "about" | "getAccounts" => {
            signal.get_profile().await
                .map(|n| n.map(|n| json!({"number": n})).unwrap_or(json!(null)))
                .map_err(|e| e.to_string())
        }
        "subscribeReceive" | "receive" | "startReceiver" => {
            signal.start_receiver().await
                .map(|_| json!({"result": "Receiver started"}))
                .map_err(|e| e.to_string())
        }
        "stopReceiver" => {
            signal.stop_receiver().await
                .map(|_| json!({"result": "Receiver stopped"}))
                .map_err(|e| e.to_string())
        }
        "listGroups" | "getGroups" => Ok(json!({"groups": []})),
        "sendTyping" | "typing" => {
            let recipient = params["recipient"].as_str().ok_or("missing recipient")?;
            signal.send_typing(recipient, false).await.map(|_| json!(null)).map_err(|e| e.to_string())
        }
        "sendReadReceipt" | "markRead" => Ok(json!(null)),
        "sendReaction" | "react" => {
            let recipient = params["recipient"].as_str().ok_or("missing recipient")?;
            let target_timestamp = params["targetTimestamp"].as_u64().ok_or("missing targetTimestamp")?;
            let emoji = params["emoji"].as_str().ok_or("missing emoji")?;
            let remove = params["remove"].as_bool().unwrap_or(false);
            
            signal.send_reaction(recipient, target_timestamp, emoji, remove).await
                .map(|_| json!({"timestamp": chrono::Utc::now().timestamp_millis()}))
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Method not found: {}", method)),
    }
}

// ==================== Message Streams ====================

/// WebSocket receive endpoint - /v1/receive/{number}
/// This is what OpenClaw uses for receiving messages!
async fn receive_messages(
    AxumState(state): AxumState<AppState>,
    _path: Path<String>,
) -> impl IntoResponse {
    // For now, redirect to SSE stream
    // In a full implementation, this would be a proper WebSocket
    Json(json!({
        "error": "Use /api/v1/events for SSE stream"
    }))
}

/// SSE stream - OpenClaw-compatible format
async fn events_stream(AxumState(state): AxumState<AppState>) -> impl IntoResponse {
    let signal = state.signal.clone();

    if !signal.is_linked() {
        return Json(json!({"error": "Not linked"})).into_response();
    }

    let mut rx = signal.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    match serde_json::to_string(&msg) {
                        Ok(json_str) => {
                            yield Ok::<_, Infallible>(Event::default().data(json_str));
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize message: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    yield Ok::<_, Infallible>(Event::default().comment("channel closed"));
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("SSE client lagged by {} messages", n);
                    yield Ok::<_, Infallible>(Event::default().comment(format!("lagged {} messages", n)));
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keepalive")
        )
        .into_response()
}
