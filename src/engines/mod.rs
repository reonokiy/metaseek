//! Search engine module
//!
//! Defines the Engine trait and provides a registry for all search engines.

mod api_key_validator;
mod loader;
mod registry;
mod traits;

// Engine implementations
pub mod ahmia;
pub mod alpinelinux;
pub mod ap_news;
pub mod apkmirror;
pub mod archlinux;
pub mod arxiv;
pub mod base;
pub mod bing;
// pub mod bloomberg;  // DISABLED: No Python implementation exists, requires validation
pub mod brave;
pub mod cachy_os;
pub mod crates;
pub mod crossref;
pub mod crunchbase;
pub mod docker_hub;
pub mod duckduckgo;
pub mod fdroid;
pub mod fortune500;
pub mod gitea;
pub mod github;
pub mod gitlab;
pub mod google;
pub mod google_scholar;
pub mod hex;
pub mod huggingface;
pub mod imdb;
pub mod lib_rs;
pub mod linkedin_companies;
pub mod metacpan;
pub mod npm;
pub mod nvd;
pub mod ollama;
pub mod openalex;
pub mod opencorporates;
pub mod pdbe;
pub mod pkg_go_dev;
pub mod pubmed;
pub mod pypi;
pub mod repology;
pub mod reuters;
pub mod scanr_structures;
pub mod sec_edgar;
pub mod semantic_scholar;
pub mod sourcehut;
pub mod stackoverflow;
pub mod voidlinux;
pub mod wikipedia;
// pub mod wsj;  // DISABLED: No Python implementation exists, requires validation
pub mod youtube;

pub use api_key_validator::{
    api_key_is_configured, extract_api_key, extract_api_key_from_params,
    generate_api_key_config_error, generate_api_key_config_error_with_instructions,
    validate_api_key_present,
};
pub use loader::EngineLoader;
pub use registry::EngineRegistry;
pub use traits::*;
