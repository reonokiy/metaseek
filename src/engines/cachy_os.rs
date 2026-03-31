//! CachyOS package search engine implementation
//!
//! Search for CachyOS packages in the official repositories.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;

use std::collections::HashMap;

/// CachyOS package search engine
pub struct CachyOS {
    base_url: String,
}

impl CachyOS {
    pub fn new() -> Self {
        Self {
            base_url: "https://packages.cachyos.org/api/search".to_string(),
        }
    }

    /// Parse CachyOS JSON package results
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let packages = json_data
            .get("packages")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in packages {
            // Get package name
            let name = item
                .get("pkg_name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Get version
            let version = item
                .get("pkg_version")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get description
            let description = item
                .get("pkg_desc")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get repository
            let repository = item
                .get("repo_name")
                .and_then(|r| r.as_str())
                .map(|r| r.to_string());

            // Get architecture
            let arch = item
                .get("pkg_arch")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get build date
            let build_date = item.get("pkg_builddate").and_then(|d| d.as_u64()).map(|d| {
                chrono::DateTime::from_timestamp(d as i64, 0)
                    .map(|dt| dt.to_string())
                    .unwrap_or_default()
            });

            // Build result URL
            let repo = repository.as_deref().unwrap_or("main");
            let arch = arch.as_deref().unwrap_or("x86_64");
            let url = format!(
                "https://packages.cachyos.org/package/{}/{}/{}",
                repo, arch, name
            );

            // Create result
            let mut result = Result::new(url, name.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(build_date) = build_date {
                result.metadata.published_date = Some(build_date);
            }

            if let Some(repo_name) = repository {
                result.metadata.tags = Some(vec![repo_name]);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for CachyOS {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for CachyOS {
    fn name(&self) -> &str {
        "cachy_os"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://packages.cachyos.org")
            .official_api(false)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "linux"]
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
        let page_size = 15;
        query_params.insert("current_page".to_string(), page.to_string());
        query_params.insert("page_size".to_string(), page_size.to_string());

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
    fn test_cachyos_request() {
        let cachyos = CachyOS::new();
        let params = RequestParams::new("curl");
        let request = cachyos.request(&params).unwrap();

        assert!(request.url.contains("packages.cachyos.org"));
        assert!(request.params.contains_key("search"));
    }
}
