//! GitHub search engine implementation
//!
//! Search for repositories, issues, and code on GitHub.
//! Supports optional API key for higher rate limits.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// GitHub search engine
pub struct GitHub {
    base_url: String,
}

impl GitHub {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.github.com/search/repositories".to_string(),
        }
    }
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GitHub {
    fn name(&self) -> &str {
        "github"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://github.com")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("sort".to_string(), "stars".to_string());
        query_params.insert("order".to_string(), "desc".to_string());

        // Pagination (GitHub API uses per_page and page)
        query_params.insert("per_page".to_string(), "10".to_string());
        query_params.insert("page".to_string(), params.pageno.to_string());

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Set the Accept header for text match highlights
        request.headers.insert(
            "Accept".to_string(),
            "application/vnd.github.preview.text-match+json".to_string(),
        );

        // GitHub API requires User-Agent
        request
            .headers
            .insert("User-Agent".to_string(), "SearXNG-RS/1.0".to_string());

        // Support optional API key for higher rate limits
        // First check for API key in engine_data (runtime override)
        let api_key = params
            .engine_data
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(key) = &api_key {
            request
                .headers
                .insert("Authorization".to_string(), format!("Bearer {}", key));
        }

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            // Check for rate limiting
            if response.status == 403 {
                return Err(anyhow::anyhow!("GitHub API rate limit exceeded"));
            }
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let json: serde_json::Value = serde_json::from_str(&response.text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let items = json
            .get("items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();

        let mut results = Vec::new();
        let mut position = 1u32;

        for item in items {
            // Get URL
            let url = item
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                continue;
            }

            // Get title (full_name = owner/repo)
            let title = item
                .get("full_name")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string();

            // Build content from language and description
            let mut content_parts = Vec::new();

            if let Some(lang) = item.get("language").and_then(|l| l.as_str()) {
                if !lang.is_empty() {
                    content_parts.push(lang.to_string());
                }
            }

            if let Some(desc) = item.get("description").and_then(|d| d.as_str()) {
                if !desc.is_empty() {
                    content_parts.push(desc.to_string());
                }
            }

            let content = if content_parts.is_empty() {
                None
            } else {
                Some(content_parts.join(" - "))
            };

            // Get thumbnail (owner's avatar)
            let thumbnail = item
                .get("owner")
                .and_then(|o| o.get("avatar_url"))
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());

            // Get author (owner's login)
            let author = item
                .get("owner")
                .and_then(|o| o.get("login"))
                .and_then(|l| l.as_str())
                .map(|s| s.to_string());

            // Get stars
            let stars = item.get("stargazers_count").and_then(|s| s.as_u64());

            // Build result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;
            result = result.with_position(position);

            if let Some(c) = content {
                result = result.with_content(c);
            }

            result.metadata.thumbnail = thumbnail;
            result.metadata.author = author;
            result.metadata.template = Some("packages.html".to_string());

            // Store stars in views field (repurposed)
            if let Some(s) = stars {
                result.metadata.views = Some(s);
            }

            results.push(result);
            position += 1;
        }

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_github_request() {
        let github = GitHub::new();
        let params = RequestParams::new("rust programming");
        let request = github.request(&params).unwrap();

        assert!(request.url.contains("github.com"));
        assert!(request.params.contains_key("q"));
    }

    #[test]
    fn test_github_request_with_api_key() {
        let github = GitHub::new();
        let mut params = RequestParams::new("rust programming");

        // Add API key to engine_data
        params
            .engine_data
            .insert("api_key".to_string(), json!("test_github_api_key"));

        let request = github.request(&params).unwrap();

        assert!(request.url.contains("github.com"));
        assert!(request
            .headers
            .get("Authorization")
            .map(|h| h.starts_with("Bearer "))
            .unwrap_or(false));
    }

    #[test]
    fn test_github_request_without_api_key() {
        let github = GitHub::new();
        let params = RequestParams::new("rust programming");
        let request = github.request(&params).unwrap();

        // Should work without API key (lower rate limits)
        assert!(request.url.contains("github.com"));
        assert!(request.headers.get("Authorization").is_none());
    }
}
