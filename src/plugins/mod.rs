//! Plugin system for Metaseek
//!
//! Plugins can hook into the search process at various points:
//! - pre_search: Before search execution
//! - on_result: For each result before aggregation
//! - post_search: After search completion

mod registry;
mod traits;

// Built-in plugins
pub mod calculator;
pub mod hash_plugin;
pub mod tracker_remover;
pub mod unit_converter;

pub use registry::PluginRegistry;
pub use traits::*;
