//! Metaseek: A privacy-respecting metasearch engine written in Rust
//!
//! This is a complete rewrite of SearXNG (originally Python) in Rust,
//! providing improved performance, memory safety, and type safety.

pub mod autocomplete;
pub mod cache;
pub mod config;
pub mod engines;
pub mod locales;
pub mod mcp;
pub mod metrics;
pub mod network;
pub mod plugins;
pub mod query;
pub mod results;
pub mod search;
pub mod web;

pub use config::Settings;
pub use engines::Engine;
pub use results::{Result as SearchResult, ResultContainer};
pub use search::{Search, SearchQuery};

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default timeout for engine requests in seconds
pub const DEFAULT_TIMEOUT: u64 = 5;

/// Maximum timeout that can be set
pub const MAX_TIMEOUT: u64 = 30;
