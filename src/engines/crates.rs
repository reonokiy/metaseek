//! crates.io search engine implementation
//!
//! Search for Rust packages on crates.io, the official Rust package registry.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// crates.io package search engine
pub struct Crates {
    base_url: String,
}

impl Crates {
    pub fn new() -> Self {
        Self {
            base_url: "https://crates.io/api/v1/crates".to_string(),
        }
    }

    /// Parse crates.io JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let crates = json_data
            .get("crates")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in crates {
            let crate_info = match item.get("crate") {
                Some(c) => c,
                None => continue,
            };

            // Get package name
            let name = crate_info
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Get description
            let description = crate_info
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get new version
            let version = crate_info
                .get("newest_version")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get homepage
            let homepage = crate_info
                .get("homepage")
                .and_then(|h| h.as_str())
                .map(|h| h.to_string());

            // Get documentation
            let documentation = crate_info
                .get("documentation")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get repository URL
            let repository = crate_info
                .get("repository")
                .and_then(|r| r.as_str())
                .map(|r| r.to_string());

            // Get authors
            let owners = crate_info
                .get("owners")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|owner| owner.get("login").and_then(|l| l.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                });

            // Get download count
            let downloads = crate_info.get("downloads").and_then(|d| d.as_u64());

            // Get recent downloads
            let _recent_downloads = crate_info.get("recent_downloads").and_then(|d| d.as_u64());

            // Get created and updated dates
            let _created_at = crate_info
                .get("created_at")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            let updated_at = crate_info
                .get("updated_at")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Build result URL
            let url = format!("https://crates.io/crates/{}", name);

            // Create result
            let mut result = Result::new(url, name.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.version = version;
            result.metadata.author = owners.map(|s| s.to_string());
            result.metadata.views = downloads;
            result.metadata.last_update = updated_at;
            result.metadata.template = Some("packages.html".to_string());

            // Add links
            if let Some(home) = homepage {
                result.metadata.homepage = Some(home);
            }
            if let Some(doc) = documentation {
                result.metadata.documentation = Some(doc);
            }
            if let Some(repo) = repository {
                result.metadata.source_code = Some(repo);
            }

            // Add tags/keywords
            if let Some(keywords) = crate_info.get("keywords").and_then(|k| k.as_array()) {
                let tags: Vec<String> = keywords
                    .iter()
                    .filter_map(|k| k.as_str().map(|s| s.to_string()))
                    .collect();
                if !tags.is_empty() {
                    result.metadata.tags = Some(tags);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Crates {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Crates {
    fn name(&self) -> &str {
        "crates"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://crates.io")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "rust"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("q".to_string(), params.query.clone());

        // Pagination
        let page = params.pageno;
        query_params.insert("page".to_string(), page.to_string());
        query_params.insert("per_page".to_string(), "10".to_string());

        // Sort by relevance
        query_params.insert("sort".to_string(), "relevance".to_string());

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

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

    #[test]
    fn test_crates_request() {
        let crates = Crates::new();
        let params = RequestParams::new("tokio");
        let request = crates.request(&params).unwrap();

        assert!(request.url.contains("crates.io"));
        assert!(request.params.contains_key("q"));
    }
}
