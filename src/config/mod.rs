//! Configuration module for Metaseek
//!
//! Handles loading and validating settings from YAML files and environment variables.

mod settings;

pub use settings::*;

use anyhow::Result;
use once_cell::sync::OnceCell;
use std::path::Path;

/// Global settings instance
static SETTINGS: OnceCell<Settings> = OnceCell::new();

/// Initialize global settings from a file
pub fn init_from_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let settings = Settings::from_file(path)?;
    SETTINGS
        .set(settings)
        .map_err(|_| anyhow::anyhow!("Settings already initialized"))?;
    Ok(())
}

/// Initialize global settings with defaults
pub fn init_default() -> Result<()> {
    let settings = Settings::default();
    SETTINGS
        .set(settings)
        .map_err(|_| anyhow::anyhow!("Settings already initialized"))?;
    Ok(())
}

/// Get a reference to the global settings
pub fn get() -> &'static Settings {
    SETTINGS.get().expect("Settings not initialized")
}

/// Check if settings have been initialized
pub fn is_initialized() -> bool {
    SETTINGS.get().is_some()
}
