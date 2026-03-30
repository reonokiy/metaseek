//! Semantic Scholar search engine implementation
//!
//! Search for academic papers using Semantic Scholar's free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use chrono::DateTime;
use std::collections::HashMap;

/// Semantic Scholar academic paper search engine
pub struct SemanticScholar {
    base_url: String,
}

impl SemanticScholar {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.semanticscholar.org/graph/v1/paper/search".to_string(),
        }
    }

    /// Parse Semantic Scholar JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let papers = json_data
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in papers {
            // Get title
            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string();

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url = item
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                continue;
            }

            // Get abstract
            let abstract_text = item
                .get("abstract")
                .and_then(|a| a.as_str())
                .map(|t| t.to_string());

            // Get authors
            let authors = item
                .get("authors")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .map(|s| s.to_string());

            // Get venue/journal
            let venue = item
                .get("venue")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get publication date
            let published_date = item
                .get("publishDate")
                .and_then(|d| d.as_str())
                .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
                .map(|dt| dt.format("%Y-%m-%d").to_string());

            // Get citation count
            let citations = item.get("citationCount").and_then(|c| c.as_u64());

            // Get DOI
            let doi = item
                .get("doi")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get open access PDF URL
            let pdf_url = item
                .get("openAccessPdf")
                .and_then(|p| p.get("url"))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get tags/fields of study
            let fields = item.get("topics").and_then(|f| f.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        f.get("name")
                            .and_then(|n| n.as_str().map(|s| s.to_string()))
                    })
                    .collect::<Vec<_>>()
            });

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Paper;

            if let Some(abstract_text) = abstract_text {
                result = result.with_content(abstract_text);
            }

            if let Some(authors) = authors {
                result.metadata.author = Some(authors);
            }

            if let Some(pub_date) = published_date {
                result.metadata.published_date = Some(pub_date);
            }

            if let Some(citations) = citations {
                result.metadata.views = Some(citations);
            }

            if let Some(doi) = doi {
                result.metadata.tags = Some(vec![format!("DOI: {}", doi)]);
            }

            if let Some(pdf) = pdf_url {
                result.metadata.img_src = Some(pdf);
            }

            if let Some(fields) = fields {
                if !fields.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.extend(fields);
                    } else {
                        result.metadata.tags = Some(fields);
                    }
                }
            }

            if let Some(venue) = venue {
                if !venue.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.insert(0, format!("Venue: {}", venue));
                    } else {
                        result.metadata.tags = Some(vec![format!("Venue: {}", venue)]);
                    }
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for SemanticScholar {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SemanticScholar {
    fn name(&self) -> &str {
        "semantic_scholar"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.semanticscholar.org/")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "scientific publications", "academic"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("query".to_string(), params.query.clone());

        // Pagination
        let offset = (params.pageno - 1) * 10;
        query_params.insert("offset".to_string(), offset.to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        // Sort by relevance
        query_params.insert("sortby".to_string(), "relevance".to_string());

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
    fn test_semantic_scholar_request() {
        let ss = SemanticScholar::new();
        let params = RequestParams::new("machine learning");
        let request = ss.request(&params).unwrap();

        assert!(request.url.contains("semanticscholar.org"));
        assert!(request.params.contains_key("query"));
    }
}
