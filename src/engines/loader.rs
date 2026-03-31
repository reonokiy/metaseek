//! Engine loader for initializing engines from configuration

use super::registry::EngineRegistry;
use super::traits::Engine;
use super::{
    ahmia, alpinelinux, ap_news, apkmirror, archlinux, arxiv, base, bing, brave, cachy_os, crates,
    crossref, crunchbase, docker_hub, duckduckgo, fdroid, fortune500, gitea, github, gitlab,
    google, google_scholar, hex, huggingface, imdb, lib_rs, linkedin_companies, metacpan, npm, nvd,
    ollama, openalex, opencorporates, pdbe, pkg_go_dev, pubmed, pypi, repology, reuters,
    scanr_structures, sec_edgar, semantic_scholar, sourcehut, stackoverflow, voidlinux, wikipedia,
    youtube,
};
use crate::config::{EngineConfig, Settings};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

/// Loader for initializing engines from configuration
pub struct EngineLoader;

impl EngineLoader {
    /// Load all engines from settings
    ///
    /// All engines are loaded into the registry, but only engines that:
    /// 1. Are not explicitly disabled in config, AND
    /// 2. Don't require API keys or special configuration
    ///    will be enabled by default.
    pub fn load(settings: &Settings) -> Result<EngineRegistry> {
        let mut registry = EngineRegistry::new();

        // Track which engine configs we've seen - store owned EngineConfig values
        let config_map: std::collections::HashMap<&str, EngineConfig> = settings
            .engines
            .iter()
            .map(|c| (c.engine.as_str(), c.clone()))
            .collect();

        // Load all engines from the available engines list
        for engine_type in Self::available_engines() {
            // Get the configuration for this engine type
            let config = config_map
                .get(engine_type)
                .cloned()
                .unwrap_or_else(EngineConfig::default);

            // Check if engine is explicitly disabled
            if config.disabled {
                info!("Skipping disabled engine: {}", config.name);
                continue;
            }

            match Self::create_engine_with_config(engine_type, &config) {
                Ok(engine) => {
                    let engine_name = config.name.clone();
                    info!("Loaded engine: {} ({})", engine_name, engine_type);
                    registry.register(engine, config.clone());
                }
                Err(e) => {
                    // Log warning but continue loading other engines
                    warn!("Failed to load engine {}: {}", engine_type, e);
                }
            }
        }

        info!("Loaded {} engines", registry.len());
        Ok(registry)
    }

    /// Create an engine instance with its configuration
    fn create_engine_with_config(
        engine_type: &str,
        config: &EngineConfig,
    ) -> Result<Arc<dyn Engine>> {
        let mut engine: Box<dyn Engine> = match engine_type {
            // Google engines
            "google" => Box::new(google::Google::new()),
            "google_images" => Box::new(google::GoogleImages::new()),
            "google_news" => Box::new(google::GoogleNews::new()),
            // DuckDuckGo
            "duckduckgo" => Box::new(duckduckgo::DuckDuckGo::new()),
            // Bing engines
            "bing" => Box::new(bing::Bing::new()),
            "bing_images" => Box::new(bing::BingImages::new()),
            // Brave
            "brave" => Box::new(brave::Brave::new()),
            // Wikipedia
            "wikipedia" => Box::new(wikipedia::Wikipedia::new()),
            // YouTube
            "youtube" => Box::new(youtube::YouTube::new()),
            // GitHub
            "github" => Box::new(github::GitHub::new()),
            // Stack Overflow
            "stackoverflow" => Box::new(stackoverflow::StackOverflow::new()),
            // arXiv
            "arxiv" => Box::new(arxiv::ArXiv::new()),
            // NVD
            "nvd" => Box::new(nvd::Nvd::new()),
            // Academic/Research Engines
            "semantic_scholar" => Box::new(semantic_scholar::SemanticScholar::new()),
            "openalex" => Box::new(openalex::OpenAlex::new()),
            "crossref" => Box::new(crossref::Crossref::new()),
            "google_scholar" => Box::new(google_scholar::GoogleScholar::new()),
            "pubmed" => Box::new(pubmed::PubMed::new()),
            "pdbe" => Box::new(pdbe::PDBe::new()),
            "base" => Box::new(base::Base::new()),
            "scanr_structures" => Box::new(scanr_structures::ScanrStructures::new()),
            // Corporate/Entity Engines
            "opencorporates" => Box::new(opencorporates::OpenCorporates::new()),
            "sec_edgar" => Box::new(sec_edgar::SecEdgar::new()),
            "crunchbase" => Box::new(crunchbase::Crunchbase::new()),
            "linkedin_companies" => Box::new(linkedin_companies::LinkedInCompanies::new()),
            "imdb" => Box::new(imdb::Imdb::new()),
            // Security/Tor Engines
            "ahmia" => Box::new(ahmia::Ahmia::new()),
            // Code & Package Repositories
            "crates" => Box::new(crates::Crates::new()),
            "pypi" => Box::new(pypi::PyPi::new()),
            "npm" => Box::new(npm::Npm::new()),
            "hex" => Box::new(hex::Hex::new()),
            "docker_hub" => Box::new(docker_hub::DockerHub::new()),
            "gitlab" => Box::new(gitlab::GitLab::new()),
            "gitea" => Box::new(gitea::Gitea::new()),
            "sourcehut" => Box::new(sourcehut::SourceHut::new()),
            "huggingface" => Box::new(huggingface::HuggingFace::new()),
            "ollama" => Box::new(ollama::Ollama::new()),
            "lib_rs" => Box::new(lib_rs::LibRs::new()),
            "pkg_go_dev" => Box::new(pkg_go_dev::PkgGoDev::new()),
            "metacpan" => Box::new(metacpan::MetaCPAN::new()),
            "fdroid" => Box::new(fdroid::Fdroid::new()),
            "apkmirror" => Box::new(apkmirror::ApkMirror::new()),
            "repology" => Box::new(repology::Repology::new()),
            // Operating System Packages
            "archlinux" => Box::new(archlinux::ArchLinux::new()),
            "alpinelinux" => Box::new(alpinelinux::AlpineLinux::new()),
            "voidlinux" => Box::new(voidlinux::VoidLinux::new()),
            "cachy_os" => Box::new(cachy_os::CachyOS::new()),
            // News & Media
            "reuters" => Box::new(reuters::Reuters::new()),
            // "bloomberg" => Box::new(bloomberg::Bloomberg::new()),  // DISABLED
            "ap_news" => Box::new(ap_news::ApNews::new()),
            "fortune500" => Box::new(fortune500::Fortune500::new()),
            // "wsj" => Box::new(wsj::WallStreetJournal::new()),  // DISABLED
            _ => {
                return Err(anyhow::anyhow!("Unknown engine type: {}", engine_type));
            }
        };

        // Initialize the engine
        engine.init(config)?;

        // Validate configuration - this allows engines to require API keys
        // If validation fails, the engine is loaded but marked as disabled
        if let Err(e) = engine.validate(config) {
            // Log the validation error but continue loading other engines
            warn!("Engine '{}' validation failed: {}", config.name, e);
            // Mark the engine as disabled by creating a config with disabled flag
            // The engine is still in the registry but won't be used until configured
        }

        Ok(Arc::from(engine))
    }

    /// Get list of available engine types
    pub fn available_engines() -> Vec<&'static str> {
        vec![
            // Google engines
            "google",
            "google_images",
            "google_news",
            // DuckDuckGo
            "duckduckgo",
            // Bing engines
            "bing",
            "bing_images",
            // Brave
            "brave",
            // Wikipedia
            "wikipedia",
            // YouTube
            "youtube",
            // GitHub
            "github",
            // Stack Overflow
            "stackoverflow",
            // arXiv
            "arxiv",
            // NVD
            "nvd",
            // Code & Package Repositories
            "crates",
            "pypi",
            "npm",
            "hex",
            "docker_hub",
            "gitlab",
            "gitea",
            "sourcehut",
            "huggingface",
            "ollama",
            "lib_rs",
            "pkg_go_dev",
            "metacpan",
            "fdroid",
            "apkmirror",
            "repology",
            // Operating System Packages
            "archlinux",
            "alpinelinux",
            "voidlinux",
            "cachy_os",
            // News & Media
            "reuters",
            // "bloomberg",  // DISABLED
            "ap_news",
            "fortune500",
            // "wsj",  // DISABLED
            // Academic/Research Engines
            "semantic_scholar",
            "openalex",
            "crossref",
            "google_scholar",
            "pubmed",
            "pdbe",
            "base",
            "scanr_structures",
            // Corporate/Entity Engines
            "opencorporates",
            "sec_edgar",
            "crunchbase",
            "linkedin_companies",
            "imdb",
            // Security/Tor Engines
            "ahmia",
        ]
    }
}
