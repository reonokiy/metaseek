//! Docker Hub search engine implementation
//!
//! Search for Docker container images on Docker Hub.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;

use std::collections::HashMap;

/// Docker Hub image search engine
pub struct DockerHub {
    base_url: String,
}

impl DockerHub {
    pub fn new() -> Self {
        Self {
            base_url: "https://hub.docker.com/v2/search/repositories/".to_string(),
        }
    }

    /// Parse Docker Hub JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let results_array = json_data
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in results_array {
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

            // Get star count
            let stars = item.get("star_count").and_then(|s| s.as_u64());

            // Get pull count
            let pulls = item.get("pull_count").and_then(|p| p.as_u64());

            // Get is official
            let is_official = item.get("is_official").and_then(|o| o.as_bool());

            // Get is verified
            let is_verified = item.get("is_verified").and_then(|v| v.as_bool());

            // Get owner name
            let owner = item
                .get("owner")
                .and_then(|o| o.as_str())
                .map(|o| o.to_string());

            // Get namespace
            let namespace = item
                .get("namespace")
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());

            // Build result URL
            let namespace_or_owner = namespace
                .or(owner.clone())
                .unwrap_or_else(|| "library".to_string());
            let _url = format!(
                "https://hub.docker.com/{}/{}",
                if is_official.unwrap_or(false) {
                    "r"
                } else {
                    "v2"
                },
                namespace_or_owner
            );

            let full_url = format!("https://hub.docker.com/{}/{}", namespace_or_owner, name);

            // Create result
            let mut result = Result::new(full_url, name.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.views = pulls;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(star_count) = stars {
                result.metadata.stars = Some(star_count);
            }

            if let Some(owner_name) = owner {
                result.metadata.author = Some(owner_name);
            }

            if is_verified.unwrap_or(false) {
                result.metadata.verified = Some(true);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for DockerHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for DockerHub {
    fn name(&self) -> &str {
        "docker_hub"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://hub.docker.com")
            .official_api(false)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "docker"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("term".to_string(), params.query.clone());

        // Pagination
        let page = params.pageno;
        query_params.insert("page".to_string(), page.to_string());
        query_params.insert("page_size".to_string(), "25".to_string());

        // Sorting
        query_params.insert("orderBy".to_string(), "name".to_string());

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
    fn test_docker_hub_request() {
        let docker_hub = DockerHub::new();
        let params = RequestParams::new("nginx");
        let request = docker_hub.request(&params).unwrap();

        assert!(request.url.contains("hub.docker.com"));
        assert!(request.params.contains_key("term"));
    }
}
