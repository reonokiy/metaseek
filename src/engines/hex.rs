//! Hex.pm search engine implementation
//!
//! Search for Elixir and Erlang packages on Hex.pm, the official package registry.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Hex.pm package search engine
pub struct Hex {
    base_url: String,
}

impl Hex {
    pub fn new() -> Self {
        Self {
            base_url: "https://hex.pm/api/packages".to_string(),
        }
    }

    /// Parse Hex.pm JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let packages = json_data.as_array().cloned().unwrap_or_default();

        let mut position = 1u32;

        for item in packages {
            // Get name
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Get description
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get HTML URL
            let html_url = item
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if html_url.is_empty() {
                continue;
            }

            // Get repository
            let repository = item
                .get("repository")
                .and_then(|r| r.as_str())
                .map(|r| r.to_string());

            // Get version
            let latest_version = item
                .get("meta")
                .and_then(|m| m.get("latest_version"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get downloads
            let total_downloads = item
                .get("meta")
                .and_then(|m| m.get("total_downloads"))
                .and_then(|d| d.as_u64());

            // Get recent downloads
            let _recent_downloads = item
                .get("meta")
                .and_then(|m| m.get("recent_downloads"))
                .and_then(|d| d.as_u64());

            // Get license
            let license = item
                .get("meta")
                .and_then(|m| m.get("licenses"))
                .and_then(|l| l.as_array())
                .and_then(|arr| arr.first())
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get authors/maintainers
            let maintainers = item
                .get("meta")
                .and_then(|m| m.get("maintainers"))
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                });

            // Get links
            let links = item
                .get("meta")
                .and_then(|m| m.get("links").and_then(|l| l.as_object()));

            let homepage = links
                .and_then(|l| l.get("home"))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            let docs = links
                .and_then(|l| l.get("docs"))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            let source = links
                .and_then(|l| l.get("source"))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Create result
            let mut result = Result::new(html_url, name.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.version = latest_version;
            result.metadata.views = total_downloads;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(maint) = maintainers {
                result.metadata.author = Some(maint);
            }

            if let Some(repo) = repository {
                result.metadata.source_code = Some(repo);
            }

            if let Some(home) = homepage {
                result.metadata.homepage = Some(home);
            }

            if let Some(doc) = docs {
                result.metadata.documentation = Some(doc);
            }

            if let Some(src) = source {
                result.metadata.source_code = Some(src);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Hex {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Hex {
    fn name(&self) -> &str {
        "hex"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://hex.pm")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "elixir", "erlang"]
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
        let per_page = 10;
        query_params.insert("page".to_string(), page.to_string());
        query_params.insert("per_page".to_string(), per_page.to_string());

        // Sort by popularity
        query_params.insert("sort".to_string(), "popular".to_string());

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
    fn test_hex_request() {
        let hex = Hex::new();
        let params = RequestParams::new("phoenix");
        let request = hex.request(&params).unwrap();

        assert!(request.url.contains("hex.pm"));
        assert!(request.params.contains_key("search"));
    }
}
