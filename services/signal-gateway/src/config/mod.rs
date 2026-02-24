//! Configuration for Signal Gateway
//!
//! Supports YAML configuration files with security settings.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub signal: SignalConfig,
    pub webhook: Option<WebhookConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL to push incoming messages to
    pub url: String,

    /// Authorization token for webhook
    pub token: String,

    /// Number of retry attempts (default: 3)
    #[serde(default = "default_webhook_retries")]
    pub retry_attempts: usize,

    /// Delay between retries in milliseconds (default: 1000)
    #[serde(default = "default_webhook_retry_delay")]
    pub retry_delay_ms: u64,
}

fn default_webhook_retries() -> usize {
    3
}
fn default_webhook_retry_delay() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "0.0.0.0:8080")
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Directory for Signal database
    pub data_dir: String,
    /// Directory for attachments
    pub attachments_dir: String,

    // Security settings - Fix #6
    /// Command channel capacity (default: 64)
    #[serde(default = "default_command_capacity")]
    pub command_channel_capacity: usize,

    /// Message broadcast capacity (default: 256)
    #[serde(default = "default_message_capacity")]
    pub message_broadcast_capacity: usize,

    /// Command timeout in milliseconds (default: 30000)
    #[serde(default = "default_command_timeout_ms")]
    pub command_timeout_ms: u64,

    /// Max sends per second for rate limiting (default: 5)
    #[serde(default = "default_max_sends_per_second")]
    pub max_sends_per_second: usize,
}

fn default_command_capacity() -> usize {
    64
}
fn default_message_capacity() -> usize {
    256
}
fn default_command_timeout_ms() -> u64 {
    30_000
}
fn default_max_sends_per_second() -> usize {
    5
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            attachments_dir: "./attachments".to_string(),
            command_channel_capacity: default_command_capacity(),
            message_broadcast_capacity: default_message_capacity(),
            command_timeout_ms: default_command_timeout_ms(),
            max_sends_per_second: default_max_sends_per_second(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: Config =
            serde_yaml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }
}
