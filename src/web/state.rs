//! Application state shared across handlers

use crate::config::Settings;
use crate::engines::EngineRegistry;
use crate::network::HttpClient;
use crate::search::Search;
use base64::Engine;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Global settings
    pub settings: Arc<Settings>,
    /// Engine registry
    pub registry: Arc<EngineRegistry>,
    /// Search executor
    pub search: Arc<Search>,
    /// Template renderer
    pub templates: Arc<super::Templates>,
    /// HTTP client for autocomplete and other requests
    pub http_client: Arc<HttpClient>,
    /// Branding configuration
    pub branding: Arc<Branding>,
}

/// Branding configuration for the search engine
#[derive(Clone, Default)]
pub struct Branding {
    /// Company name
    pub name: Option<String>,
    /// Logo URL (external or local path)
    pub logo: Option<String>,
    /// Tagline or description
    pub tagline: Option<String>,
    /// Color scheme accent
    pub accent_color: Option<String>,
    /// Embedded logo as base64 data URI (if logo was a local file)
    pub logo_data_uri: Option<String>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        settings: Settings,
        registry: EngineRegistry,
        client: HttpClient,
    ) -> anyhow::Result<Self> {
        let settings = Arc::new(settings);
        let registry = Arc::new(registry);
        let http_client = Arc::new(client.clone());
        let search = Arc::new(Search::new(client, registry.clone()));
        let templates = Arc::new(super::Templates::new()?);

        // All configuration sources have already been merged into Settings.
        let name = settings.branding.name.clone();
        let logo = settings.branding.logo.clone();
        let tagline = settings.branding.tagline.clone();
        let accent_color = settings.branding.accent_color.clone();

        // If logo is a local file path, read it and convert to base64 data URI
        let logo_data_uri = logo.as_ref().and_then(|l| {
            if l.starts_with('/') || l.starts_with("./") || l.starts_with("../") {
                // It's a local file path
                read_logo_to_data_uri(l).ok()
            } else {
                // It's a remote URL
                None
            }
        });

        let branding = Arc::new(Branding {
            name,
            logo,
            tagline,
            accent_color,
            logo_data_uri,
        });

        Ok(Self {
            settings,
            registry,
            search,
            templates,
            http_client,
            branding,
        })
    }

    /// Get instance name
    pub fn instance_name(&self) -> &str {
        &self.settings.general.instance_name
    }

    /// Check if instance is public
    pub fn is_public(&self) -> bool {
        self.settings.server.public_instance
    }

    /// Get configured autocomplete backend name
    pub fn autocomplete_backend(&self) -> Option<&str> {
        self.settings.search.autocomplete.as_deref()
    }

    /// Get branding name
    pub fn branding_name(&self) -> Option<&str> {
        self.branding.name.as_deref()
    }

    /// Get branding logo URL
    pub fn branding_logo(&self) -> Option<&str> {
        self.branding.logo.as_deref()
    }

    /// Check if logo is a local path (starts with /)
    pub fn is_local_logo(&self) -> bool {
        self.branding
            .logo
            .as_deref()
            .is_some_and(|logo| logo.starts_with('/'))
    }

    /// Get logo URL - resolves local paths relative to static assets
    pub fn get_logo_url(&self) -> Option<String> {
        self.branding.logo.as_ref().map(|logo| {
            // If it's a local path, ensure it starts with /
            if logo.starts_with('/') {
                logo.clone()
            } else {
                // Remote URL - return as-is
                logo.clone()
            }
        })
    }

    /// Get branding tagline
    pub fn branding_tagline(&self) -> Option<&str> {
        self.branding.tagline.as_deref()
    }

    /// Get branding accent color
    pub fn branding_accent_color(&self) -> Option<&str> {
        self.branding.accent_color.as_deref()
    }
}

/// Read a logo file and convert it to a base64 data URI
fn read_logo_to_data_uri(path: &str) -> anyhow::Result<String> {
    use std::io::Read;

    // Determine the file extension to set the correct MIME type
    let mime_type = if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else {
        "image/png" // default
    };

    // Read the file content
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Convert to base64
    let base64_content = base64::engine::general_purpose::STANDARD.encode(&buffer);

    // Construct the data URI
    Ok(format!("data:{};base64,{}", mime_type, base64_content))
}
