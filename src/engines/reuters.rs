//! Reuters news search engine implementation
//!
//! Provides access to Reuters news articles via their official JSON API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use serde_json::json;

/// Reuters news search engine
pub struct Reuters {
    base_url: String,
}

impl Reuters {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.reuters.com".to_string(),
        }
    }

    /// Parse Reuters news results from JSON API
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        // Reuters API returns data in "result" -> "articles"
        let articles = json_data
            .get("result")
            .and_then(|r| r.get("articles"))
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for article in articles {
            // Extract fields from Reuters JSON
            let title = article
                .get("web")
                .and_then(|t| t.as_str())
                .unwrap_or("No title")
                .to_string();

            let url = article
                .get("canonical_url")
                .and_then(|u| u.as_str())
                .map(|u| format!("{}{}", self.base_url, u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            let content = article
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            let kicker = article
                .get("kicker")
                .and_then(|k| k.get("name"))
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::News;

            if let Some(desc) = content {
                result = result.with_content(desc);
            }

            if let Some(kicker_name) = kicker {
                result.metadata.tags = Some(vec![kicker_name]);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Reuters {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Reuters {
    fn name(&self) -> &str {
        "reuters"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.reuters.com")
            .official_api(true)
            .api_key_required(false)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["news", "financial"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        // Reuters API expects a JSON query string
        let sort_order = "relevance";
        let offset = (params.pageno - 1) * 20; // 20 results per page

        let query_args = json!({
            "keyword": params.query,
            "offset": offset,
            "orderby": sort_order,
            "size": 20,
            "website": "reuters"
        });

        let query_string = serde_json::to_string(&query_args).unwrap_or_default();
        let encoded_query = urlencoding::encode(&query_string);

        let url = format!(
            "{}/pf/api/v3/content/fetch/articles-by-search-v2?query={}",
            self.base_url, encoded_query
        );

        let mut request = EngineRequest::get(&url);
        request.headers.insert(
            "Accept".to_string(),
            "application/json".to_string(),
        );
        request.headers.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (compatible; Metaseek)".to_string(),
        );

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
    fn test_reuters_request() {
        let reuters = Reuters::new();
        let params = RequestParams::new("technology news");
        let request = reuters.request(&params).unwrap();

        assert!(request.url.contains("reuters.com"));
        assert!(request.url.contains("articles-by-search-v2"));
    }
}