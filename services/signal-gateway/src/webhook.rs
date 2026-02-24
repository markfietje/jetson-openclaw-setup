//! Webhook client for pushing Signal messages to OpenClaw
//!
//! When a Signal message is received, this module forwards it to OpenClaw's webhook endpoint.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWebhookPayload {
    /// The message text from Signal
    pub message: String,
    
    /// Human-readable name for the webhook
    pub name: String,
    
    /// Agent ID to route to
    pub agent_id: String,
    
    /// Channel to deliver response to
    pub channel: String,
    
    /// Recipient identifier (phone number)
    pub to: String,
    
    /// Whether to deliver the response
    pub deliver: bool,
    
    /// Wake mode (immediate or next heartbeat)
    pub wake_mode: String,
}

#[derive(Debug, Clone)]
pub struct WebhookClient {
    url: String,
    token: String,
    retry_attempts: usize,
    retry_delay: Duration,
    client: reqwest::Client,
}

impl WebhookClient {
    pub fn new(url: String, token: String, retry_attempts: usize, retry_delay_ms: u64) -> Self {
        Self {
            url,
            token,
            retry_attempts,
            retry_delay: Duration::from_millis(retry_delay_ms),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Forward a Signal message to OpenClaw webhook
    pub async fn forward_message(
        &self,
        sender_phone: &str,
        sender_uuid: &str,
        message_text: &str,
        your_phone: &str,
    ) -> Result<()> {
        let payload = SignalWebhookPayload {
            message: format!("{} sent: {}", sender_phone, message_text),
            name: "Signal".to_string(),
            agent_id: "main".to_string(),
            channel: "signal".to_string(),
            to: sender_phone.to_string(),
            deliver: true,
            wake_mode: "now".to_string(),
        };

        self.send_with_retry(&payload).await
    }

    /// Send payload to webhook with retry logic
    async fn send_with_retry(&self, payload: &SignalWebhookPayload) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..self.retry_attempts {
            match self.try_send(payload).await {
                Ok(_) => {
                    tracing::info!("Webhook sent successfully to {}", self.url);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "Webhook attempt {}/{} failed: {}",
                        attempt + 1,
                        self.retry_attempts,
                        e
                    );
                    last_error = Some(e);

                    // Don't sleep after the last attempt
                    if attempt < self.retry_attempts - 1 {
                        sleep(self.retry_delay).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Webhook failed after {} attempts: {}",
            self.retry_attempts,
            last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"))
        ))
    }

    /// Single attempt to send webhook
    async fn try_send(&self, payload: &SignalWebhookPayload) -> Result<()> {
        let response = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Webhook returned {}: {}",
                status,
                body
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = SignalWebhookPayload {
            message: "Test message".to_string(),
            name: "Signal".to_string(),
            agent_id: "main".to_string(),
            channel: "signal".to_string(),
            to: "+353833006868".to_string(),
            deliver: true,
            wake_mode: "now".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("Test message"));
        assert!(json.contains("signal"));
    }
}
