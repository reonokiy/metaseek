//! Ahmia search engine implementation
//!
//! Search for .onion (Tor hidden service) sites using Ahmia's free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Ahmia Tor hidden service search engine
pub struct Ahmia {
    base_url: String,
}

impl Ahmia {
    pub fn new() -> Self {
        Self {
            base_url: "https://ahmia.fi/api/search/".to_string(),
        }
    }

    /// Parse Ahmia JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let hits = json_data
            .get("hits")
            .and_then(|h| h.get("hits"))
            .and_then(|h| h.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in hits {
            // Get URL
            let url = item
                .get("path")
                .and_then(|p| p.as_str())
                .map(|p| format!("http://{}", p.trim_start_matches('/')))
                .or_else(|| {
                    item.get("origin_url")
                        .and_then(|u| u.as_str())
                        .map(|u| u.to_string())
                })
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get title
            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("No title")
                .to_string();

            // Get description
            let content = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get favicon
            let favicon = item
                .get("favicon")
                .and_then(|f| f.as_str())
                .map(|f| f.to_string());

            // Get verification status
            let verified = item.get("verified").and_then(|v| v.as_bool());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Default;

            if let Some(desc) = content {
                result = result.with_content(desc);
            }

            if let Some(favicon) = favicon {
                result.metadata.thumbnail = Some(favicon);
            }

            if verified.unwrap_or(false) {
                result.metadata.is_official = true;
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
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["general", "tor", "security"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("search".to_string(), params.query.clone());

        // Pagination
        query_params.insert("page".to_string(), params.pageno.to_string());

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
    fn test_ahmia_request() {
        let ahmia = Ahmia::new();
        let params = RequestParams::new("search");
        let request = ahmia.request(&params).unwrap();

        assert!(request.url.contains("ahmia.fi"));
        assert!(request.params.contains_key("search"));
    }
}
