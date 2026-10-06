//! Engine Registry
//!
//! This file contains:
//! 1. `ALL_ENGINES`: Static list for `build.rs` to generate category maps.
//! 2. `EngineRegistry`: Runtime registry holding actual engine instances and configs.

use super::traits::Engine;
use crate::config::EngineConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Static list of engines and categories for build.rs
pub const ALL_ENGINES: &[(&str, &[&str])] = &[
    ("google", &["general", "web"]),
    ("google_images", &["images"]),
    ("google_news", &["news"]),
    ("duckduckgo", &["general", "web"]),
    ("bing", &["general", "web"]),
    ("bing_images", &["images"]),
    ("brave", &["general", "web"]),
    ("wikipedia", &["general", "web"]),
    ("youtube", &["videos"]),
    ("github", &["it", "code"]),
    ("stackoverflow", &["it", "code"]),
    ("arxiv", &["science", "academic"]),
    ("pubmed", &["science", "academic", "medical"]),
    ("reuters", &["news", "media"]),
    ("ahmia", &["tor", "onions", "security"]),
    ("nvd", &["it", "security", "vulnerabilities"]),
    ("crates", &["it", "packages", "rust"]),
    ("pypi", &["it", "packages", "python"]),
    ("npm", &["it", "packages", "javascript"]),
    ("docker_hub", &["it", "packages", "docker"]),
    ("pkg_go_dev", &["it", "packages", "go"]),
    ("fdroid", &["it", "packages", "android"]),
    ("apkmirror", &["it", "packages", "android"]),
    ("metacpan", &["it", "packages", "perl"]),
    ("lib_rs", &["it", "documentation", "rust"]),
    ("repology", &["it", "packages", "linux"]),
    ("archlinux", &["it", "packages", "linux"]),
    ("alpinelinux", &["it", "packages", "linux"]),
    ("voidlinux", &["it", "packages", "linux"]),
    ("cachy_os", &["it", "packages", "linux"]),
    ("bloomberg", &["news", "business", "financial"]),
    ("ap_news", &["news", "media"]),
    ("fortune500", &["news", "business", "corporate"]),
    ("wsj", &["news", "business", "financial"]),
    (
        "opencorporates",
        &["corporate", "business", "companies", "entities"],
    ),
    ("imdb", &["movies", "corporate"]),
    (
        "sec_edgar",
        &["corporate", "business", "financial", "sec", "filings"],
    ),
    (
        "crunchbase",
        &["corporate", "business", "startups", "investments"],
    ),
    (
        "linkedin_companies",
        &["corporate", "business", "companies"],
    ),
    ("semantic_scholar", &["science", "academic"]),
    ("openalex", &["science", "academic"]),
    ("crossref", &["science", "academic"]),
    ("google_scholar", &["science", "academic"]),
    ("pdbe", &["science", "biology", "chemistry"]),
    ("base", &["science", "academic", "research"]),
    ("scanr_structures", &["science", "academic"]),
    ("huggingface", &["it", "ai", "machine_learning"]),
    ("ollama", &["it", "ai", "machine_learning"]),
    ("gitea", &["it", "code"]),
    ("gitlab", &["it", "code"]),
    ("sourcehut", &["it", "code", "privacy"]),
    ("startpage", &["general", "web"]),
    ("qwant", &["general", "web"]),
    ("yandex", &["general", "web"]),
    ("baidu", &["general", "web"]),
    ("searx", &["general", "web"]),
    ("ecosia", &["general", "web"]),
    ("mojeek", &["general", "web"]),
    ("duckduckgo_images", &["images"]),
    ("bing_videos", &["videos"]),
    ("dailymotion", &["videos"]),
    ("vimeo", &["videos"]),
    ("flickr", &["images"]),
    ("unsplash", &["images"]),
    ("pexels", &["images"]),
    ("pixabay", &["images"]),
    ("wallhaven", &["images"]),
    ("reddit", &["social", "news"]),
    ("twitter", &["social", "news"]),
    ("facebook", &["social", "news"]),
    ("instagram", &["social", "news"]),
    ("tiktok", &["social", "news"]),
    ("mastodon", &["social", "news"]),
    ("lemmy", &["social", "news"]),
    ("lobste.rs", &["it", "news"]),
    ("hackernews", &["it", "news"]),
    ("slashdot", &["it", "news"]),
    ("techcrunch", &["news", "technology"]),
    ("wired", &["news", "technology"]),
    ("theverge", &["news", "technology"]),
    ("arstechnica", &["news", "technology"]),
    ("engadget", &["news", "technology"]),
    ("cnet", &["news", "technology"]),
    ("zdnet", &["news", "technology"]),
    ("venturebeat", &["news", "technology"]),
    ("mashable", &["news", "technology"]),
    ("gizmodo", &["news", "technology"]),
    ("lifehacker", &["news", "technology"]),
    ("makeuseof", &["news", "technology"]),
    ("howtogeek", &["news", "technology"]),
    ("digitaltrends", &["news", "technology"]),
    ("techradar", &["news", "technology"]),
    ("tomsguide", &["news", "technology"]),
    ("pcmag", &["news", "technology"]),
    ("cnn", &["news", "politics"]),
    ("bbc", &["news", "politics"]),
    ("reuters_news", &["news", "politics"]),
    ("ap_news_politics", &["news", "politics"]),
    ("nytimes", &["news", "politics"]),
    ("washingtonpost", &["news", "politics"]),
    ("wallstreetjournal", &["news", "business"]),
    ("forbes", &["news", "business"]),
    ("bloomberg_news", &["news", "business"]),
    ("cnbc", &["news", "business"]),
    ("marketwatch", &["news", "business"]),
    ("businessinsider", &["news", "business"]),
    ("fortune", &["news", "business"]),
    ("economist", &["news", "business"]),
    ("financialtimes", &["news", "business"]),
    ("investopedia", &["news", "business"]),
    ("seekingalpha", &["news", "business"]),
    ("motleyfool", &["news", "business"]),
    ("zacks", &["news", "business"]),
    ("ycharts", &["news", "business"]),
    ("stocktwits", &["news", "business"]),
    ("finviz", &["news", "business"]),
    ("tradingview", &["news", "business"]),
    ("investing", &["news", "business"]),
    ("morningstar", &["news", "business"]),
    ("barrons", &["news", "business"]),
    ("dowjones", &["news", "business"]),
    ("reuters_markets", &["news", "business"]),
    ("bloomberg_markets", &["news", "business"]),
    ("wsj_markets", &["news", "business"]),
    ("cnbc_markets", &["news", "business"]),
    ("marketwatch_markets", &["news", "business"]),
    ("forbes_markets", &["news", "business"]),
    ("fortune_markets", &["news", "business"]),
    ("economist_markets", &["news", "business"]),
    ("financialtimes_markets", &["news", "business"]),
    ("investopedia_markets", &["news", "business"]),
    ("seekingalpha_markets", &["news", "business"]),
    ("motleyfool_markets", &["news", "business"]),
    ("zacks_markets", &["news", "business"]),
    ("ycharts_markets", &["news", "business"]),
    ("stocktwits_markets", &["news", "business"]),
    ("finviz_markets", &["news", "business"]),
    ("tradingview_markets", &["news", "business"]),
    ("investing_markets", &["news", "business"]),
    ("morningstar_markets", &["news", "business"]),
    ("barrons_markets", &["news", "business"]),
    ("dowjones_markets", &["news", "business"]),
];

/// Runtime engine registry holding actual engine instances and configs
pub struct EngineRegistry {
    engines: HashMap<String, Arc<dyn Engine + Send + Sync>>,
    categories: HashMap<String, Vec<String>>,
    configs: HashMap<String, EngineConfig>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
            categories: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn register(&mut self, engine: Arc<dyn Engine + Send + Sync>, config: EngineConfig) {
        let name = engine.name().to_string();
        let categories = engine
            .categories()
            .iter()
            .map(|s: &&str| s.to_string())
            .collect::<Vec<_>>();

        self.engines.insert(name.clone(), engine);
        self.configs.insert(name.clone(), config);

        for cat in categories {
            self.categories.entry(cat).or_default().push(name.clone());
        }
    }

    pub fn get_timeout(&self, engine_name: &str, default_timeout: f64) -> f64 {
        // Try to get timeout from engine config, otherwise use default
        self.configs
            .get(engine_name)
            .and_then(|c| c.timeout)
            .unwrap_or(default_timeout)
    }

    pub fn get_engine(&self, name: &str) -> Option<&(dyn Engine + Send + Sync)> {
        self.engines
            .get(name)
            .map(|e: &Arc<dyn Engine + Send + Sync>| e.as_ref() as &(dyn Engine + Send + Sync))
    }

    pub fn get_engine_mut(&mut self, name: &str) -> Option<&mut (dyn Engine + Send + Sync)> {
        self.engines
            .get_mut(name)
            .map(|e: &mut Arc<dyn Engine + Send + Sync>| {
                Arc::get_mut(e).unwrap() as &mut (dyn Engine + Send + Sync)
            })
    }

    pub fn get_by_category(&self, category: &str) -> Vec<&(dyn Engine + Send + Sync)> {
        self.categories
            .get(category)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|n: &String| {
                        self.engines
                            .get(n)
                            .map(|e: &Arc<dyn Engine + Send + Sync>| {
                                e.as_ref() as &(dyn Engine + Send + Sync)
                            })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_engine_names_by_category(&self, category: &str) -> Vec<String> {
        self.categories.get(category).cloned().unwrap_or_default()
    }

    pub fn get_config(&self, name: &str) -> Option<&EngineConfig> {
        self.configs.get(name)
    }

    pub fn get_weight(&self, name: &str) -> f64 {
        self.configs.get(name).map(|c| c.weight).unwrap_or(1.0)
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Engine + Send + Sync>> {
        self.engines.get(name)
    }

    pub fn names(&self) -> Vec<String> {
        self.engines.keys().cloned().collect()
    }

    pub fn category_names(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.engines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.engines.is_empty()
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate the category map from the static ALL_ENGINES constant at runtime.
/// This replaces the need for a build script.
pub fn get_category_map() -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for (name, cats) in ALL_ENGINES {
        for cat in *cats {
            map.entry(cat.to_string())
                .or_default()
                .push(name.to_string());
        }
    }

    map
}
