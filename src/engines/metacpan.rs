//! MetaCPAN package search engine implementation
//!
//! Search for Perl packages on MetaCPAN.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// MetaCPAN package search engine
pub struct MetaCPAN {
    base_url: String,
}

impl MetaCPAN {
    pub fn new() -> Self {
        Self {
            base_url: "https://fastapi.metacpan.org/v1/file/_search".to_string(),
        }
    }

    /// Parse MetaCPAN JSON search results
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
            // Get source document
            let source = match item.get("_source") {
                Some(s) => s,
                None => continue,
            };

            // Get documentation/module name
            let module = source
                .get("documentation")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            if module.is_empty() {
                continue;
            }

            // Get abstract
            let abstract_text = source
                .get("abstract")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get version
            let version = source
                .get("version")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get author
            let author = source
                .get("author")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get license
            let license = source
                .get("license")
                .and_then(|l| l.as_array())
                .and_then(|arr| arr.first())
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get distribution
            let distribution = source
                .get("distribution")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get released timestamp
            let released = source
                .get("released")
                .and_then(|r| r.as_u64())
                .and_then(|r| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(r as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                });

            // Get module URL
            let url = format!("https://metacpan.org/pod/{}", module);

            // Create result
            let mut result = Result::new(url, module.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(abs) = abstract_text {
                result = result.with_content(abs);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(auth) = author {
                result.metadata.author = Some(auth);
            }

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(dist) = distribution {
                result.metadata.tags = Some(vec![dist]);
            }

            if let Some(rel) = released {
                result.metadata.published_date = Some(rel);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for MetaCPAN {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for MetaCPAN {
    fn name(&self) -> &str {
        "metacpan"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://metacpan.org")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "perl"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        use serde_json::json;

        let mut query_params = HashMap::new();

        // Build search query
        let query_body = json!({
            "query": {
                "multi_match": {
                    "query": params.query.clone(),
                    "type": "most_fields",
                    "fields": ["documentation", "documentation.*"],
                    "analyzer": "camelcase"
                }
            },
            "filter": {
                "bool": {
                    "must": [
                        { "exists": { "field": "documentation" } },
                        { "term": { "status": "latest" } },
                        { "term": { "indexed": 1 } },
                        { "term": { "authorized": 1 } }
                    ]
                }
            },
            "sort": [
                { "_score": { "order": "desc" } },
                { "date": { "order": "desc" } }
            ],
            "_source": ["documentation", "abstract"],
            "from": (params.pageno - 1) * 20,
            "size": 20
        });

        query_params.insert("query".to_string(), query_body.to_string());

        let mut request = EngineRequest::post(&self.base_url);
        request.params = query_params;
        request
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());

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
    fn test_metacpan_request() {
        let metacpan = MetaCPAN::new();
        let params = RequestParams::new("Digest::SHA");
        let request = metacpan.request(&params).unwrap();

        assert!(request.url.contains("metacpan.org"));
        assert!(request.method == HttpMethod::Post);
    }
}
