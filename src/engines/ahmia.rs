//! Ahmia search engine implementation
//!
//! Search for .onion (Tor hidden service) sites using Ahmia's HTML interface.
//! Note: This engine requires Tor network access (socks5 proxy) to reach the .onion domain.
//! It implements a token handshake to bypass Ahmia's anti-bot measures.
//!
//! **Configuration Requirement**: Ensure your system or the HTTP client is configured
//! to route traffic through a Tor proxy. The engine reads proxy settings from environment
//! variables: `ALL_PROXY`, `HTTP_PROXY`, `HTTPS_PROXY`, or `SOCKS_PROXY`.
//! Example: `export ALL_PROXY=socks5h://127.0.0.1:9050`

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use dashmap::DashMap;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

// Cache for tokens: (name_token, value_token, expiration_timestamp)
static AHMIA_TOKEN_CACHE: LazyLock<DashMap<String, (String, String, u64)>> = LazyLock::new(DashMap::new);

/// Ahmia Tor hidden service search engine
pub struct Ahmia {
    base_url: String,
}

impl Ahmia {
    pub fn new() -> Self {
        Self {
            base_url: "http://juhanurmihxlp77nkq76byazcldy2hlmovfu2epvl5ankdibsot4csyd.onion".to_string(),
        }
    }

    /// Build a reqwest client with proxy support from environment variables
    fn build_client() -> Option<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(true) // Needed for .onion self-signed certs
            .user_agent("Mozilla/5.0 (compatible; SearXNG-RS/1.0)");

        // Check for proxy environment variables
        let proxy_url = std::env::var("ALL_PROXY")
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("HTTPS_PROXY"))
            .or_else(|_| std::env::var("SOCKS_PROXY"))
            .ok()?;

        // Skip empty proxy URLs
        if proxy_url.trim().is_empty() {
            return None; // No proxy configured
        }

        // Use Proxy::all which supports socks5:// URLs when the 'socks' feature is enabled
        let proxy = reqwest::Proxy::all(&proxy_url).ok()?;
        builder = builder.proxy(proxy);

        builder.build().ok()
    }

    /// Fetch tokens from the Ahmia homepage (Async)
    async fn fetch_tokens(&self) -> Option<(String, String)> {
        let client = Self::build_client()?;
        let response = client.get(&self.base_url).send().await.ok()?;
        let html = response.text().await.ok()?;

        self.parse_tokens(&html)
    }

    /// Parse tokens from HTML
    fn parse_tokens(&self, html: &str) -> Option<(String, String)> {
        let document = Html::parse_document(html);
        let name_selector = Selector::parse(r#"input[type="hidden"][name]"#).ok()?;
        
        let mut name_token = None;
        let mut value_token = None;

        for element in document.select(&name_selector) {
            if let Some(name) = element.value().attr("name") {
                if let Some(value) = element.value().attr("value") {
                    if name_token.is_none() {
                        name_token = Some(name.to_string());
                        value_token = Some(value.to_string());
                    }
                }
            }
        }

        name_token.and_then(|n| value_token.map(|v| (n, v)))
    }

    /// Get tokens from cache (Synchronous)
    fn get_cached_tokens(&self) -> Option<(String, String)> {
        let cache_key = "ahmia_tokens";
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(entry) = AHMIA_TOKEN_CACHE.get(cache_key) {
            let (_, _, exp) = entry.value();
            if now < *exp {
                let (n, v, _) = entry.value().clone();
                return Some((n, v));
            }
        }
        None
    }

    /// Parse Ahmia HTML response
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let mut results = Vec::new();
        let document = Html::parse_document(html);

        let result_selector = Selector::parse(r#"li.result"#).unwrap();
        let url_selector = Selector::parse(r#"h4 a[href]"#).unwrap();
        let title_selector = Selector::parse(r#"h4 a"#).unwrap();
        let content_selector = Selector::parse(r#"p"#).unwrap();

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            let raw_url = element
                .select(&url_selector)
                .next()
                .and_then(|a| a.value().attr("href"))
                .unwrap_or("");

            // Clean up URL - Ahmia redirects through its own URL
            let url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
                let cleaned = if let Ok(parsed) = Url::parse(raw_url) {
                    if let Some(query) = parsed.query() {
                        query.split('&')
                            .find(|p| p.starts_with("redirect_url="))
                            .and_then(|p| p.split('=').nth(1))
                            .map(|u| u.to_string())
                            .unwrap_or_else(|| raw_url.to_string())
                    } else {
                        raw_url.to_string()
                    }
                } else {
                    raw_url.to_string()
                };
                cleaned
            } else {
                format!("{}{}", self.base_url, raw_url)
            };

            if url.is_empty() {
                continue;
            }

            let title = element
                .select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "No title".to_string());

            let content = element
                .select(&content_selector)
                .next()
                .map(|c| c.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Default;

            if let Some(desc) = content {
                result = result.with_content(desc);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Ahmia {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Ahmia {
    fn name(&self) -> &str {
        "ahmia"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://ahmia.fi/")
            .official_api(false)
            .api_key_required(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["tor"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());

        // Time range support
        if let Some(ref time_range) = params.time_range {
            let days = match time_range {
                crate::query::TimeRange::Day => 1,
                crate::query::TimeRange::Week => 7,
                crate::query::TimeRange::Month => 30,
                crate::query::TimeRange::Year => 365,
            };
            query_params.insert("d".to_string(), days.to_string());
        }

        // Pagination
        let page = params.pageno;
        if page > 1 {
            query_params.insert("page".to_string(), page.to_string());
        }

        // Inject cached tokens if available
        if let Some((name_token, value_token)) = self.get_cached_tokens() {
            query_params.insert(name_token, value_token);
        } else {
            tracing::warn!("Ahmia tokens not found in cache. Ensure Tor proxy is configured (ALL_PROXY=socks5h://127.0.0.1:9050) and tokens have been fetched.");
        }

        let url = format!(
            "{}/search/?{}",
            self.base_url,
            serde_urlencoded::to_string(&query_params).unwrap_or_default()
        );

        let mut request = EngineRequest::get(&url);
        request.headers.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (compatible; SearXNG-RS)".to_string(),
        );

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let results = self.parse_results(&response.text);
        Ok(EngineResults::with_results(results))
    }
}

// Helper for async token refresh
impl Ahmia {
    pub async fn refresh_tokens(&self) -> Option<(String, String)> {
        let tokens = self.fetch_tokens().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let (name_token, value_token) = tokens.clone();
        AHMIA_TOKEN_CACHE.insert(
            "ahmia_tokens".to_string(),
            (name_token, value_token, now + 3600),
        );
        Some(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ahmia_request() {
        let ahmia = Ahmia::new();
        let params = RequestParams::new("search");
        let request = ahmia.request(&params).unwrap();

        assert!(request.url.contains("juhanurmihxlp77nkq76byazcldy2hlmovfu2epvl5ankdibsot4csyd.onion"));
        assert!(request.url.contains("search"));
    }

    #[test]
    fn test_build_client_respects_proxy_env() {
        // This test verifies that build_client returns Some if a proxy is set,
        // and None if no proxy is set.
        // It does NOT manipulate environment variables; it relies on the current shell state.
        
        let client = Ahmia::build_client();
        
        // If ALL_PROXY (or others) is set in the environment, client should be Some
        // If not set, client should be None
        // We simply assert that the function returns a value consistent with the env
        let has_proxy = std::env::var("ALL_PROXY").or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("HTTPS_PROXY")).or_else(|_| std::env::var("SOCKS_PROXY"))
            .map(|v| !v.trim().is_empty()).unwrap_or(false);
            
        if has_proxy {
            assert!(client.is_some(), "Client should be built when proxy env var is set");
        } else {
            assert!(client.is_none(), "Client should be None when no proxy env var is set");
        }
    }
}