//! OpenClaw-compatible message types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Incoming Signal message (OpenClaw-compatible)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalMessage {
    pub account: Option<String>,
    pub envelope: SignalEnvelope,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalEnvelope {
    #[serde(rename = "sourceNumber", skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "sourceUuid", skip_serializing_if = "Option::is_none")]
    pub source_uuid: Option<String>,
    #[serde(rename = "sourceName", skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_device: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "dataMessage", skip_serializing_if = "Option::is_none")]
    pub data_message: Option<SignalDataMessage>,
    #[serde(rename = "syncMessage", skip_serializing_if = "Option::is_none")]
    pub sync_message: Option<serde_json::Value>,
    #[serde(rename = "reactionMessage", skip_serializing_if = "Option::is_none")]
    pub reaction_message: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalDataMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<SignalAttachment>>,
    #[serde(rename = "groupInfo", skip_serializing_if = "Option::is_none")]
    pub group_info: Option<SignalGroupInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<SignalQuote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<SignalMention>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<SignalReactionMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalReactionMessage {
    pub emoji: Option<String>,
    #[serde(rename = "targetAuthor")]
    pub target_author: Option<String>,
    #[serde(rename = "targetAuthorUuid")]
    pub target_author_uuid: Option<String>,
    #[serde(rename = "targetSentTimestamp")]
    pub target_sent_timestamp: Option<i64>,
    #[serde(rename = "isRemove")]
    pub is_remove: Option<bool>,
    #[serde(rename = "groupInfo")]
    pub group_info: Option<SignalGroupInfo>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalAttachment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "contentType", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalGroupInfo {
    #[serde(rename = "groupId", skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(rename = "groupName", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalQuote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SignalMention {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<i32>,
}

/// Configuration for the Signal manager
#[derive(Clone, Debug)]
pub struct ManagerConfig {
    pub db_path: String,
    pub command_channel_capacity: usize,
    pub message_broadcast_capacity: usize,
    pub command_timeout_ms: u64,
    pub max_sends_per_second: usize,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            db_path: "signal.db".to_string(),
            command_channel_capacity: 64,
            message_broadcast_capacity: 256,
            command_timeout_ms: 30_000,
            max_sends_per_second: 5,
        }
    }
}

impl ManagerConfig {
    pub fn new(db_path: String, _attachments_dir: String) -> Self {
        Self {
            db_path,
            ..Default::default()
        }
    }
}
