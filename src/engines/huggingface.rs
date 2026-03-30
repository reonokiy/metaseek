//! Hugging Face search engine implementation
//!
//! Search for models, datasets, and spaces on Hugging Face.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Hugging Face search engine
pub struct HuggingFace {
    base_url: String,
    endpoint: String,
}

impl HuggingFace {
    pub fn new() -> Self {
        Self {
            base_url: "https://huggingface.co".to_string(),
            endpoint: "models".to_string(),
        }
    }

    /// Set custom endpoint (models, datasets, or spaces)
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_string();
        self
    }

    /// Parse Hugging Face JSON search results
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let items = json_data.as_array().cloned().unwrap_or_default();

        let mut position = 1u32;

        for item in items {
            // Get ID/name
            let id = item
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string();

            if id.is_empty() {
                continue;
            }

            // Get full ID (username/model)
            let full_id = item
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string();

            // Get URL
            let url = item
                .get("id")
                .and_then(|i| i.as_str())
                .map(|i| format!("https://huggingface.co/{}", i))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get description
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get downloads
            let downloads = item.get("downloads").and_then(|d| d.as_u64());

            // Get likes
            let likes = item.get("likes").and_then(|l| l.as_u64());

            // Get trending
            let trending_score = item.get("trendingScore").and_then(|t| t.as_f64());

            // Get tags
            let tags = item.get("tags").and_then(|t| t.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

            // Get library
            let library = item
                .get("library")
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get language
            let languages = item.get("language").and_then(|l| l.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|l| l.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

            // Get pipeline
            let pipeline = item
                .get("pipeline_tag")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            // Get author
            let author = item
                .get("author")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get created at
            let created_at = item
                .get("createdAt")
                .and_then(|c| c.as_str())
                .map(|c| c.to_string());

            // Get last modified
            let last_modified = item
                .get("lastModified")
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get private status
            let private = item.get("private").and_then(|p| p.as_bool());

            // Get disabled status
            let disabled = item.get("disabled").and_then(|d| d.as_bool());

            // Create result
            let mut result = Result::new(url.clone(), full_id, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.template = Some("packages.html".to_string());

            if let Some(downloads) = downloads {
                result.metadata.views = Some(downloads);
            }

            if let Some(likes) = likes {
                result.metadata.stars = Some(likes);
            }

            if let Some(trending) = trending_score {
                result.metadata.trending_score = Some(trending);
            }

            if let Some(tags_vec) = tags {
                if !tags_vec.is_empty() {
                    result.metadata.tags = Some(tags_vec);
                }
            }

            if let Some(lib) = library {
                result.metadata.library = Some(lib);
            }

            if let Some(langs) = languages {
                if !langs.is_empty() {
                    if let Some(existing_tags) = &mut result.metadata.tags {
                        existing_tags.extend(langs);
                    } else {
                        result.metadata.tags = Some(langs);
                    }
                }
            }

            if let Some(pipeline) = pipeline {
                result.metadata.pipeline = Some(pipeline);
            }

            if let Some(author) = author {
                result.metadata.author = Some(author);
            }

            if let Some(created) = created_at {
                result.metadata.published_date = Some(created);
            }

            if let Some(modified) = last_modified {
                result.metadata.last_update = Some(modified);
            }

            if private == Some(true) {
                result.metadata.private = Some(true);
            }

            if disabled == Some(true) {
                result.metadata.disabled = Some(true);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for HuggingFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for HuggingFace {
    fn name(&self) -> &str {
        "huggingface"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://huggingface.co")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos", "ml", "ai"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("search".to_string(), params.query.clone());

        // Sort by downloads (default)
        query_params.insert("direction".to_string(), "-1".to_string());

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
    fn test_huggingface_request() {
        let hf = HuggingFace::new();
        let params = RequestParams::new("transformers");
        let request = hf.request(&params).unwrap();

        assert!(request.url.contains("huggingface.co"));
        assert!(request.params.contains_key("search"));
    }
}
