//! Application state shared across handlers

use crate::config::Settings;
use crate::engines::EngineRegistry;
use crate::network::HttpClient;
use crate::search::Search;
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
    /// Logo URL
    pub logo: Option<String>,
    /// Tagline or description
    pub tagline: Option<String>,
    /// Color scheme accent
    pub accent_color: Option<String>,
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
        let branding = Arc::new(Branding {
            name: settings.branding.name.clone(),
            logo: settings.branding.logo.clone(),
            tagline: settings.branding.tagline.clone(),
            accent_color: settings.branding.accent_color.clone(),
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

    /// Get branding tagline
    pub fn branding_tagline(&self) -> Option<&str> {
        self.branding.tagline.as_deref()
    }

    /// Get branding accent color
    pub fn branding_accent_color(&self) -> Option<&str> {
        self.branding.accent_color.as_deref()
    }
}
