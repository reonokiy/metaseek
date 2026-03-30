//! Result type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    /// The URL of the result
    pub url: String,
    /// Parsed URL for easier manipulation
    #[serde(skip)]
    pub parsed_url: Option<Url>,
    /// The title of the result
    pub title: String,
    /// Content snippet/description
    pub content: Option<String>,
    /// Engine that returned this result
    pub engine: String,
    /// All engines that returned this result (after merging)
    #[serde(default)]
    pub engines: HashSet<String>,
    /// Positions in each engine's results
    #[serde(default)]
    pub positions: Vec<u32>,
    /// Calculated relevance score
    #[serde(default)]
    pub score: f64,
    /// Category of the result
    pub category: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: ResultMetadata,
    /// Result type
    #[serde(default)]
    pub result_type: ResultType,
}

impl Result {
    /// Create a new result
    pub fn new(url: String, title: String, engine: String) -> Self {
        let parsed_url = Url::parse(&url).ok();
        let mut engines = HashSet::new();
        engines.insert(engine.clone());

        Self {
            url,
            parsed_url,
            title,
            content: None,
            engine,
            engines,
            positions: vec![],
            score: 0.0,
            category: None,
            metadata: ResultMetadata::default(),
            result_type: ResultType::Default,
        }
    }

    /// Add content to the result
    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Add a position
    pub fn with_position(mut self, position: u32) -> Self {
        self.positions.push(position);
        self
    }

    /// Get the hostname from the URL
    pub fn hostname(&self) -> Option<&str> {
        self.parsed_url.as_ref().and_then(|u| u.host_str())
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: &Result) {
        self.engines.extend(other.engines.clone());
        self.positions.extend(other.positions.clone());

        // Use content from other if we don't have any
        if self.content.is_none() && other.content.is_some() {
            self.content = other.content.clone();
        }
    }

    /// Calculate the score based on positions and engine weights
    pub fn calculate_score(&mut self, engine_weights: &std::collections::HashMap<String, f64>) {
        let mut weight = 1.0;

        for engine in &self.engines {
            if let Some(w) = engine_weights.get(engine) {
                weight *= w;
            }
        }

        weight *= self.engines.len() as f64;

        self.score = self.positions.iter().map(|&pos| weight / pos as f64).sum();
    }
}

/// Additional result metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultMetadata {
    /// Thumbnail URL
    pub thumbnail: Option<String>,
    /// Image URL (for image results)
    pub img_src: Option<String>,
    /// Template to use for rendering
    pub template: Option<String>,
    /// Author name
    pub author: Option<String>,
    /// Published date
    pub published_date: Option<String>,
    /// File type
    pub file_type: Option<String>,
    /// File size
    pub file_size: Option<String>,
    /// Duration (for videos)
    pub duration: Option<String>,
    /// View count
    pub views: Option<u64>,
    /// Iframe source
    pub iframe_src: Option<String>,
    /// Audio source
    pub audio_src: Option<String>,
    /// Is official result
    pub is_official: bool,
    /// Version string
    pub version: Option<String>,
    /// License information
    pub license: Option<String>,
    /// Tags for categorization
    pub tags: Option<Vec<String>>,
    /// Source code URL
    pub source_code: Option<String>,
    /// SSH URL for repositories
    pub ssh_url: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Documentation URL
    pub documentation: Option<String>,
    /// Last update timestamp
    pub last_update: Option<String>,
    /// Repository architecture
    pub architecture: Option<String>,
    /// Package name
    pub package_name: Option<String>,
    /// Star count
    pub stars: Option<u64>,
    /// Fork count
    pub forks: Option<u64>,
    /// Verified status
    pub verified: Option<bool>,
    /// Severity level (for security vulnerabilities)
    pub severity: Option<String>,
    /// CVSS score (for security vulnerabilities)
    pub cvss_score: Option<f64>,
    /// Modified date (for security vulnerabilities)
    pub modified_date: Option<String>,
    /// Archived status (for git repositories)
    pub archived: Option<bool>,
    /// Mirror status (for git repositories)
    pub mirror: Option<bool>,
    /// Trending score (for Hugging Face models)
    pub trending_score: Option<f64>,
    /// Machine learning library (for Hugging Face models)
    pub library: Option<String>,
    /// Pipeline type (for Hugging Face models)
    pub pipeline: Option<String>,
    /// Private status (for Hugging Face models)
    pub private: Option<bool>,
    /// Disabled status (for Hugging Face models)
    pub disabled: Option<bool>,
}

/// Type of result
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    #[default]
    Default,
    Image,
    Video,
    Map,
    News,
    Paper,
    File,
    Code,
    Answer,
    InfoBox,
    Security,
    Corporate,
}

/// An answer result (calculator, definition, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// The answer text
    pub answer: String,
    /// Source engine
    pub engine: String,
    /// URL for more info
    pub url: Option<String>,
}

impl Answer {
    pub fn new(answer: String, engine: String) -> Self {
        Self {
            answer,
            engine,
            url: None,
        }
    }
}

/// A suggestion for related searches
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Suggestion {
    pub text: String,
    pub engine: String,
}

/// A spelling correction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Correction {
    pub text: String,
    pub engine: String,
}

/// An infobox result (Wikipedia sidebar, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoBox {
    /// Infobox ID
    pub id: String,
    /// Title
    pub title: String,
    /// Content (HTML)
    pub content: Option<String>,
    /// Image URL
    pub img_src: Option<String>,
    /// Source URL
    pub url: Option<String>,
    /// Source engine
    pub engine: String,
    /// Key-value attributes
    #[serde(default)]
    pub attributes: Vec<(String, String)>,
    /// Related URLs
    #[serde(default)]
    pub urls: Vec<(String, String)>,
}

/// Engine response timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    /// Engine name
    pub engine: String,
    /// Response time in milliseconds
    pub time_ms: u64,
    /// Number of results returned
    pub result_count: usize,
}

/// Engine error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineError {
    Timeout,
    NetworkError,
    HttpError(u16),
    ParseError,
    AccessDenied,
    Captcha,
    TooManyRequests,
    RateLimited,
    ServerError,
    Suspended,
    MissingApiKey,
    Unknown,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Request timed out"),
            Self::NetworkError => write!(f, "Network error"),
            Self::HttpError(code) => write!(f, "HTTP error: {}", code),
            Self::ParseError => write!(f, "Failed to parse response"),
            Self::AccessDenied => write!(f, "Access denied"),
            Self::Captcha => write!(f, "CAPTCHA required"),
            Self::TooManyRequests => write!(f, "Too many requests"),
            Self::RateLimited => write!(f, "API rate limit exceeded"),
            Self::ServerError => write!(f, "Server error"),
            Self::Suspended => write!(f, "Engine suspended"),
            Self::MissingApiKey => write!(f, "API key required but not configured"),
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

/// An unresponsive engine record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresponsiveEngine {
    pub name: String,
    pub error: EngineError,
}
