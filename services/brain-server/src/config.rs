//! Configuration constants for brain-server

#![allow(dead_code)]

pub const MODEL_ID: &str = "minishlab/potion-retrieval-32M";
pub const DEFAULT_K: usize = 5;
pub const MAX_K: usize = 100;
pub const SERVER_VERSION: &str = "0.8.1";

pub const MAX_REQUEST_SIZE: usize = 1024 * 1024;
pub const MAX_QUERY_LENGTH: usize = 2000;

pub const REQUEST_TIMEOUT_SECS: u64 = 30;
pub const SEARCH_TIMEOUT_SECS: u64 = 8;
pub const HEALTH_TIMEOUT_SECS: u64 = 3;
pub const SHUTDOWN_DRAIN_SECS: u64 = 60;

pub const POOL_MAX_SIZE: u32 = 20;
pub const POOL_MIN_IDLE: u32 = 2;
pub const POOL_CONNECTION_TIMEOUT_SECS: u64 = 30;
pub const POOL_MAX_LIFETIME_SECS: u64 = 300;
pub const POOL_IDLE_TIMEOUT_SECS: u64 = 60;

pub const CONTENT_MAX_LENGTH: usize = 1_000_000;
pub const TITLE_MAX_LENGTH: usize = 500;

pub const CONNECTION_WATCHDOG_INTERVAL_SECS: u64 = 30;
pub const CONNECTION_WATCHDOG_THRESHOLD_SECS: u64 = 300;

pub const ENTITY_NAME_MAX_LENGTH: usize = 100;
pub const TRAVERSE_MAX_DEPTH: u8 = 3;

pub const CORS_DEFAULT_ORIGINS: &str = "http://localhost:3000,http://localhost:8080";
pub const CORS_DEFAULT_METHODS: &str = "GET,POST,PUT,DELETE,OPTIONS";
pub const CORS_DEFAULT_HEADERS: &str = "content-type,authorization";
pub const CORS_MAX_AGE_SECS: u64 = 3600;

pub const SEARCH_BATCH_SIZE: usize = 500;
