//! GitLab search engine implementation
//!
//! Search for GitLab repositories and projects.
//! Supports optional API key for private instance access and higher rate limits.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// GitLab repository search engine
pub struct GitLab {
    base_url: String,
}

impl GitLab {
    pub fn new() -> Self {
        Self {
            base_url: "https://gitlab.com/api/v4/search".to_string(),
        }
    }

    /// Set custom base URL for GitLab instance
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Parse GitLab JSON repository results
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let repositories = json_data.as_array().cloned().unwrap_or_default();

        let mut position = 1u32;

        for item in repositories {
            // Get web URL
            let web_url = item
                .get("web_url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if web_url.is_empty() {
                continue;
            }

            // Get name (already captured in web_url above)
            let _name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default();

            // Get path
            let _path = item
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or_default();

            // Get description
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get path with namespace
            let path_with_namespace = item
                .get("path_with_namespace")
                .and_then(|p| p.as_str())
                .unwrap_or_default()
                .to_string();

            // Get visibility
            let _visibility = item
                .get("visibility")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get SSH URL
            let ssh_url = item
                .get("ssh_url_to_repo")
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get HTTP URL
            let http_url = item
                .get("http_url_to_repo")
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get last activity
            let last_activity = item
                .get("last_activity_at")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get created at
            let created_at = item
                .get("created_at")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get default branch
            let default_branch = item
                .get("default_branch")
                .and_then(|b| b.as_str())
                .map(|b| b.to_string());

            // Get stars count
            let stars = item.get("star_count").and_then(|s| s.as_u64());

            // Get forks count
            let forks = item.get("forks_count").and_then(|f| f.as_u64());

            // Get open issues count
            let _open_issues = item.get("open_issues_count").and_then(|i| i.as_i64());

            // Get archived status
            let archived = item.get("archived").and_then(|a| a.as_bool());

            // Get mirror status
            let mirror = item.get("mirror").and_then(|m| m.as_bool());

            // Get namespace
            let namespace = item
                .get("namespace")
                .and_then(|n| n.get("path"))
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            // Get author/maintainer
            let author = item
                .get("owner")
                .and_then(|o| o.get("username"))
                .or_else(|| item.get("owner").and_then(|o| o.get("name")))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get tags
            let tags = item.get("tag_list").and_then(|t| t.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

            // Create result
            let mut result = Result::new(
                web_url,
                path_with_namespace.clone(),
                self.name().to_string(),
            );
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.version = default_branch;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(ns) = namespace {
                if !ns.is_empty() {
                    result.metadata.author = Some(ns);
                }
            }

            if let Some(author) = author {
                if !author.is_empty() {
                    result.metadata.author = Some(author);
                }
            }

            if let Some(stars) = stars {
                result.metadata.stars = Some(stars);
            }

            if let Some(forks) = forks {
                result.metadata.forks = Some(forks);
            }

            if let Some(last_activity) = last_activity {
                result.metadata.last_update = Some(last_activity);
            }

            if let Some(created) = created_at {
                result.metadata.published_date = Some(created);
            }

            if let Some(tags_vec) = tags {
                if !tags_vec.is_empty() {
                    result.metadata.tags = Some(tags_vec);
                }
            }

            if let Some(ssh) = ssh_url {
                result.metadata.ssh_url = Some(ssh);
            }

            if let Some(http) = http_url {
                result.metadata.source_code = Some(http);
            }

            if archived == Some(true) {
                result.metadata.archived = Some(true);
            }

            if mirror == Some(true) {
                result.metadata.mirror = Some(true);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for GitLab {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GitLab {
    fn name(&self) -> &str {
        "gitlab"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://gitlab.com")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos", "code"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("search".to_string(), params.query.clone());

        // Pagination
        let page = params.pageno;
        query_params.insert("page".to_string(), page.to_string());

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Support optional API key for higher rate limits and private instance access
        // First check for API key in engine_data (runtime override)
        let api_key = params
            .engine_data
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(key) = &api_key {
            request
                .headers
                .insert("PRIVATE-TOKEN".to_string(), key.clone());
        }

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let json_data: serde_json::Value = serde_json::from_str(&response.text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let results = self.parse_results(&json_data);

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gitlab_request() {
        let gitlab = GitLab::new();
        let params = RequestParams::new("rust programming");
        let request = gitlab.request(&params).unwrap();

        assert!(request.url.contains("gitlab.com"));
        assert!(request.params.contains_key("search"));
    }

    #[test]
    fn test_gitlab_request_with_api_key() {
        let gitlab = GitLab::new();
        let mut params = RequestParams::new("rust programming");

        // Add API key to engine_data
        params
            .engine_data
            .insert("api_key".to_string(), json!("test_gitlab_api_key"));

        let request = gitlab.request(&params).unwrap();

        assert!(request.headers.get("PRIVATE-TOKEN").is_some());
    }

    #[test]
    fn test_gitlab_request_without_api_key() {
        let gitlab = GitLab::new();
        let params = RequestParams::new("rust programming");
        let request = gitlab.request(&params).unwrap();

        // Should work without API key (lower rate limits)
        assert!(request.url.contains("gitlab.com"));
        assert!(request.headers.get("PRIVATE-TOKEN").is_none());
    }
}
