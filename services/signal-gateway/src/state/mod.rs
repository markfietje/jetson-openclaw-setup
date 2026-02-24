//! Application State

use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::signal::{ManagerConfig, SignalHandle, SignalWorker, SignalMessage};
use crate::webhook::WebhookClient;

#[derive(Clone)]
pub struct AppState {
    pub signal: SignalHandle,
    pub webhook: Option<Arc<WebhookClient>>,
    _webhook_task: Option<Arc<JoinHandle<()>>>,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self> {
        let manager_config = ManagerConfig {
            db_path: format!("{}/signal.db", config.signal.data_dir),
            command_channel_capacity: config.signal.command_channel_capacity,
            message_broadcast_capacity: config.signal.message_broadcast_capacity,
            command_timeout_ms: config.signal.command_timeout_ms,
            max_sends_per_second: config.signal.max_sends_per_second,
        };
        
        let worker = SignalWorker::spawn(manager_config)?;
        let signal = worker.handle();
        std::mem::forget(worker);
        
        // Create webhook client if configured
        let webhook = if let Some(webhook_config) = &config.webhook {
            Some(Arc::new(WebhookClient::new(
                webhook_config.url.clone(),
                webhook_config.token.clone(),
                webhook_config.retry_attempts,
                webhook_config.retry_delay_ms,
            )))
        } else {
            None
        };
        
        // Start webhook forwarding task if webhook is configured
        let _webhook_task = if webhook.is_some() {
            let signal_clone = signal.clone();
            let webhook_clone = webhook.clone().unwrap();
            let task = tokio::spawn(async move {
                Self::webhook_forwarder(signal_clone, webhook_clone).await;
            });
            Some(Arc::new(task))
        } else {
            None
        };
        
        Ok(Self { signal, webhook, _webhook_task })
    }
    
    pub async fn init_signal(&self) -> Result<bool> {
        let loaded = self.signal.load_registered().await?;
        if loaded {
            if let Err(e) = self.signal.start_receiver().await {
                tracing::warn!("Failed to auto-start receiver: {}", e);
            }
        }
        Ok(loaded)
    }
    
    /// Background task that forwards Signal messages to OpenClaw webhook
    async fn webhook_forwarder(signal: SignalHandle, webhook: Arc<WebhookClient>) {
        let mut rx = signal.subscribe();
        
        tracing::info!("Webhook forwarder started");
        
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    // Only forward messages with text content
                    if let Some(dm) = &msg.envelope.data_message {
                        if let Some(text) = &dm.message {
                            // Get sender info
                            let sender_uuid = msg.envelope.source_uuid.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
                            let account = msg.account.as_ref().map(|s| s.as_str()).unwrap_or("unknown");
                            
                            tracing::info!("Forwarding message from {} to webhook", sender_uuid);
                            
                            // Forward to webhook (fire and forget, log errors)
                            if let Err(e) = webhook.forward_message(
                                sender_uuid,
                                sender_uuid,
                                text,
                                account,
                            ).await {
                                tracing::error!("Webhook forward failed: {}", e);
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::warn!("Webhook forwarder: channel closed");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Webhook forwarder lagged by {} messages", n);
                }
            }
        }
        
        tracing::info!("Webhook forwarder stopped");
    }
}
