//! Settings structures for SearXNG-RS configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main settings structure matching SearXNG's settings.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub search: SearchSettings,
    pub server: ServerSettings,
    pub outgoing: OutgoingSettings,
    pub engines: Vec<EngineConfig>,
    pub plugins: PluginsSettings,
    pub ui: UiSettings,
    pub redis: Option<RedisSettings>,
    pub branding: BrandingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            search: SearchSettings::default(),
            server: ServerSettings::default(),
            outgoing: OutgoingSettings::default(),
            engines: default_engines(),
            plugins: PluginsSettings::default(),
            ui: UiSettings::default(),
            redis: None,
            branding: BrandingSettings::default(),
        }
    }
}

impl Settings {
    /// Load settings from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let settings: Settings = serde_yaml::from_str(&content)?;
        Ok(settings)
    }

    /// Merge with environment variables (SEARXNG_* prefix)
    pub fn merge_env(&mut self) {
        if let Ok(val) = std::env::var("SEARXNG_DEBUG") {
            self.general.debug = val.parse().unwrap_or(false);
        }
        if let Ok(val) = std::env::var("SEARXNG_SECRET_KEY") {
            self.server.secret_key = val;
        }
        if let Ok(val) = std::env::var("SEARXNG_PORT") {
            if let Ok(port) = val.parse() {
                self.server.port = port;
            }
        }
        if let Ok(val) = std::env::var("SEARXNG_BIND_ADDRESS") {
            self.server.bind_address = val;
        }
        if let Ok(val) = std::env::var("SEARXNG_BASE_URL") {
            self.server.base_url = Some(val);
        }
    }

    /// Get engine config by name
    pub fn get_engine(&self, name: &str) -> Option<&EngineConfig> {
        self.engines.iter().find(|e| e.name == name)
    }

    /// Get all enabled engines
    pub fn enabled_engines(&self) -> Vec<&EngineConfig> {
        self.engines.iter().filter(|e| !e.disabled).collect()
    }

    /// Get engines by category
    pub fn engines_by_category(&self, category: &str) -> Vec<&EngineConfig> {
        self.engines
            .iter()
            .filter(|e| !e.disabled && e.categories.contains(&category.to_string()))
            .collect()
    }
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// Enable debug mode
    pub debug: bool,
    /// Instance name displayed in UI
    pub instance_name: String,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Privacy policy URL
    pub privacypolicy_url: Option<String>,
    /// Donation URL
    pub donation_url: Option<String>,
    /// Contact URL
    pub contact_url: Option<String>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            debug: false,
            instance_name: "SearXNG".to_string(),
            enable_metrics: true,
            privacypolicy_url: None,
            donation_url: None,
            contact_url: None,
        }
    }
}

/// Search behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    /// Safe search level: 0 = off, 1 = moderate, 2 = strict
    pub safe_search: u8,
    /// Autocomplete backend (google, duckduckgo, etc.)
    pub autocomplete: Option<String>,
    /// Default language code
    pub default_lang: String,
    /// Time to ban engine after failure (seconds)
    pub ban_time_on_fail: u64,
    /// Maximum ban time (seconds)
    pub max_ban_time_on_fail: u64,
    /// Suspended times tracking
    pub suspended_times: SuspendedTimes,
    /// Default search categories
    pub default_categories: Vec<String>,
    /// Maximum number of results per page
    pub max_page: u32,
    /// Formats available for export
    pub formats: Vec<String>,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            safe_search: 0,
            autocomplete: None,
            default_lang: "auto".to_string(),
            ban_time_on_fail: 5,
            max_ban_time_on_fail: 120,
            suspended_times: SuspendedTimes::default(),
            default_categories: vec!["general".to_string()],
            max_page: 10,
            formats: vec![
                "html".to_string(),
                "json".to_string(),
                "csv".to_string(),
                "rss".to_string(),
            ],
        }
    }
}

/// Suspended times configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SuspendedTimes {
    /// Suspend time for network errors
    pub network_error: u64,
    /// Suspend time for HTTP errors
    pub http_error: u64,
    /// Suspend time for CAPTCHA detection
    pub captcha: u64,
    /// Suspend time for too many requests
    pub too_many_requests: u64,
}

impl Default for SuspendedTimes {
    fn default() -> Self {
        Self {
            network_error: 120,
            http_error: 60,
            captcha: 3600,
            too_many_requests: 600,
        }
    }
}

/// Server settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    /// Server port
    pub port: u16,
    /// Bind address
    pub bind_address: String,
    /// Base URL for the instance
    pub base_url: Option<String>,
    /// Enable rate limiter
    pub limiter: bool,
    /// Public instance mode
    pub public_instance: bool,
    /// Secret key for sessions
    pub secret_key: String,
    /// Enable image proxy
    pub image_proxy: bool,
    /// HTTP protocol version
    pub http_protocol_version: String,
    /// Method to determine real IP
    pub real_ip_method: RealIpMethod,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 8888,
            bind_address: "127.0.0.1".to_string(),
            base_url: None,
            limiter: false,
            public_instance: false,
            secret_key: generate_secret_key(),
            image_proxy: false,
            http_protocol_version: "1.1".to_string(),
            real_ip_method: RealIpMethod::default(),
        }
    }
}

/// Method to determine real client IP
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealIpMethod {
    /// Use X-Forwarded-For header
    XForwardedFor,
    /// Use X-Real-IP header
    XRealIp,
    /// Use connection IP directly
    #[default]
    Connection,
}

/// Outgoing request settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutgoingSettings {
    /// Default request timeout in seconds
    pub request_timeout: f64,
    /// Maximum request timeout
    pub max_request_timeout: Option<f64>,
    /// User agent string (none = random)
    pub useragent_suffix: Option<String>,
    /// Pool connections count
    pub pool_connections: usize,
    /// Pool max size
    pub pool_maxsize: usize,
    /// Enable IPv6
    pub enable_ipv6: bool,
    /// Verify SSL certificates
    pub verify_ssl: bool,
    /// Proxy settings
    pub proxies: ProxySettings,
    /// Extra headers to send
    pub extra_headers: HashMap<String, String>,
}

impl Default for OutgoingSettings {
    fn default() -> Self {
        Self {
            request_timeout: 5.0,
            max_request_timeout: Some(30.0),
            useragent_suffix: None,
            pool_connections: 100,
            pool_maxsize: 20,
            enable_ipv6: true,
            verify_ssl: true,
            proxies: ProxySettings::default(),
            extra_headers: HashMap::new(),
        }
    }
}

/// Proxy settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxySettings {
    pub http: Option<String>,
    pub https: Option<String>,
    pub all: Option<String>,
}

/// Individual engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// Engine name (unique identifier)
    pub name: String,
    /// Engine module to use
    pub engine: String,
    /// Categories this engine belongs to
    pub categories: Vec<String>,
    /// Short name for UI
    pub shortcut: String,
    /// Whether engine is disabled
    pub disabled: bool,
    /// Custom timeout for this engine
    pub timeout: Option<f64>,
    /// Engine weight for scoring
    pub weight: f64,
    /// Display name
    pub display_name: Option<String>,
    /// API key if required
    pub api_key: Option<String>,
    /// Additional engine-specific settings
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            engine: String::new(),
            categories: vec!["general".to_string()],
            shortcut: String::new(),
            disabled: false,
            timeout: None,
            weight: 1.0,
            display_name: None,
            api_key: None,
            extra: HashMap::new(),
        }
    }
}

/// Plugin settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsSettings {
    /// Enabled plugins
    pub enabled: Vec<String>,
    /// Disabled plugins
    pub disabled: Vec<String>,
}

impl Default for PluginsSettings {
    fn default() -> Self {
        Self {
            enabled: vec![
                "hash_plugin".to_string(),
                "self_info".to_string(),
                "tracker_url_remover".to_string(),
            ],
            disabled: vec![],
        }
    }
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Default theme
    pub default_theme: String,
    /// Available themes
    pub themes: Vec<String>,
    /// Default locale
    pub default_locale: String,
    /// Results per page
    pub results_per_page: u32,
    /// Infinite scroll
    pub infinite_scroll: bool,
    /// Center alignment
    pub center_alignment: bool,
    /// Query in title
    pub query_in_title: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            default_theme: "simple".to_string(),
            themes: vec!["simple".to_string()],
            default_locale: "en".to_string(),
            results_per_page: 10,
            infinite_scroll: false,
            center_alignment: false,
            query_in_title: true,
        }
    }
}

/// Redis/Valkey settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisSettings {
    pub url: String,
}

/// Branding settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrandingSettings {
    /// Brand name to display
    pub name: Option<String>,
    /// Logo URL
    pub logo: Option<String>,
    /// Tagline or description
    pub tagline: Option<String>,
    /// Accent color for UI customization
    pub accent_color: Option<String>,
}

impl Default for BrandingSettings {
    fn default() -> Self {
        Self {
            name: None,
            logo: None,
            tagline: None,
            accent_color: None,
        }
    }
}

/// Generate a random secret key
fn generate_secret_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

/// Default engine configurations
fn default_engines() -> Vec<EngineConfig> {
    vec![
        EngineConfig {
            name: "google".to_string(),
            engine: "google".to_string(),
            categories: vec!["general".to_string(), "web".to_string()],
            shortcut: "g".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "duckduckgo".to_string(),
            engine: "duckduckgo".to_string(),
            categories: vec!["general".to_string(), "web".to_string()],
            shortcut: "ddg".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "bing".to_string(),
            engine: "bing".to_string(),
            categories: vec!["general".to_string(), "web".to_string()],
            shortcut: "bi".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "brave".to_string(),
            engine: "brave".to_string(),
            categories: vec!["general".to_string(), "web".to_string()],
            shortcut: "br".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "wikipedia".to_string(),
            engine: "wikipedia".to_string(),
            categories: vec!["general".to_string()],
            shortcut: "wp".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "google images".to_string(),
            engine: "google_images".to_string(),
            categories: vec!["images".to_string()],
            shortcut: "gi".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "bing images".to_string(),
            engine: "bing_images".to_string(),
            categories: vec!["images".to_string()],
            shortcut: "bii".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "youtube".to_string(),
            engine: "youtube".to_string(),
            categories: vec!["videos".to_string()],
            shortcut: "yt".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "google news".to_string(),
            engine: "google_news".to_string(),
            categories: vec!["news".to_string()],
            shortcut: "gn".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "arxiv".to_string(),
            engine: "arxiv".to_string(),
            categories: vec!["science".to_string()],
            shortcut: "arx".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "github".to_string(),
            engine: "github".to_string(),
            categories: vec!["it".to_string()],
            shortcut: "gh".to_string(),
            ..Default::default()
        },
        EngineConfig {
            name: "stackoverflow".to_string(),
            engine: "stackoverflow".to_string(),
            categories: vec!["it".to_string()],
            shortcut: "so".to_string(),
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.port, 8888);
        assert!(!settings.general.debug);
        assert!(!settings.engines.is_empty());
    }

    #[test]
    fn test_engine_lookup() {
        let settings = Settings::default();
        let google = settings.get_engine("google");
        assert!(google.is_some());
        assert_eq!(google.unwrap().shortcut, "g");
    }
}
