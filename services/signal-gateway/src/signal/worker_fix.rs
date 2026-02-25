//! Phone Number to ACI Resolution Patch for Signal Gateway
//!
//! This patch adds phone number resolution capability to fix sending.
//!
//! The Problem:
//! - Signal uses ACI (UUID) internally for all messaging
//! - OpenClaw sends phone numbers (e.g., +15551234567)
//! - ServiceId::parse_from_service_id_string() fails on phone numbers
//!
//! The Solution:
//! 1. Cache phone → ACI mappings from incoming messages
//! 2. Resolve phone numbers using Manager's capabilities
//! 3. Fallback: accept both UUID and phone number formats

use anyhow::{Context, Result};
use parking_lot::Mutex;
use presage::libsignal_service::protocol::ServiceId;
use std::collections::HashMap;
use std::sync::Arc;

/// Phone number to ACI cache
#[derive(Clone)]
pub struct RecipientCache {
    cache: Arc<Mutex<HashMap<String, String>>>, // phone_number -> uuid
}

impl RecipientCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Insert a phone → UUID mapping (from incoming messages)
    pub fn insert(&self, phone: String, uuid: String) {
        tracing::info!("[CACHE] Mapping {} -> {}", phone, uuid);
        self.cache.lock().insert(phone, uuid);
    }

    /// Lookup UUID by phone number
    pub fn get(&self, phone: &str) -> Option<String> {
        self.cache.lock().get(phone).cloned()
    }

    /// Check if recipient is already a UUID
    pub fn is_uuid(&self, recipient: &str) -> bool {
        // UUID format: 8-4-4-4-12 hex digits
        recipient.len() == 36 && recipient.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    }

    /// Resolve recipient to ServiceId
    /// Input can be:
    /// - UUID format (already resolved)
    /// - Phone number (look up in cache)
    pub fn resolve(&self, recipient: &str) -> Result<String> {
        // Already a UUID? Return as-is
        if self.is_uuid(recipient) {
            tracing::info!("[RESOLVE] Already a UUID: {}", recipient);
            return Ok(recipient.to_string());
        }

        // Phone number? Look up in cache
        if let Some(uuid) = self.get(recipient) {
            tracing::info!("[RESOLVE] Cache hit: {} -> {}", recipient, uuid);
            return Ok(uuid);
        }

        // Not found - this is the problem!
        tracing::warn!("[RESOLVE] Unknown recipient: {}", recipient);
        anyhow::bail!("Cannot resolve recipient: {} (not in cache, not a UUID)", recipient)
    }
}

/// Extension to process incoming messages and extract phone numbers
///
/// When Signal sends messages, the metadata includes:
/// - sender.raw_uuid() - the ACI
/// - sender.e164() - the phone number (if available)
///
/// We need to capture this mapping to enable replies.
pub fn extract_phone_mapping(content: &presage::libsignal_service::content::Content) -> Option<(String, String)> {
    use presage::libsignal_service::sender::Sender;

    let sender = &content.metadata.sender;

    // Get phone number (e164 format)
    let phone = sender.e164().to_string();

    // Get UUID
    let uuid = sender.raw_uuid().to_string();

    if phone.is_empty() || uuid.is_empty() {
        return None;
    }

    Some((phone, uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_uuid() {
        let cache = RecipientCache::new();
        assert!(cache.is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!cache.is_uuid("+15551234567"));
        assert!(!cache.is_uuid("not-a-uuid"));
    }

    #[test]
    fn test_cache_lookup() {
        let cache = RecipientCache::new();
        cache.insert("+15551234567".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string());
        assert_eq!(cache.get("+15551234567"), Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }

    #[test]
    fn test_resolve_uuid() {
        let cache = RecipientCache::new();
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(cache.resolve(uuid).unwrap(), uuid);
    }

    #[test]
    fn test_resolve_cached_phone() {
        let cache = RecipientCache::new();
        cache.insert("+15551234567".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string());
        assert_eq!(cache.resolve("+15551234567").unwrap(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_resolve_unknown() {
        let cache = RecipientCache::new();
        assert!(cache.resolve("+15551234567").is_err());
    }
}
