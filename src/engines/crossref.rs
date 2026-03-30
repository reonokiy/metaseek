//! Crossref search engine implementation
//!
//! Search for scholarly articles and metadata using Crossref's free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Crossref academic paper search engine
pub struct Crossref {
    base_url: String,
}

impl Crossref {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.crossref.org/works".to_string(),
        }
    }

    /// Parse Crossref JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let items = json_data
            .get("message")
            .and_then(|m| m.get("items"))
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in items {
            // Skip component items (files published with papers)
            if item.get("type").and_then(|t| t.as_str()) == Some("component") {
                continue;
            }

            // Get title
            let title = item
                .get("title")
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.first())
                .and_then(|t| t.as_str())
                .unwrap_or("No title")
                .to_string();

            // Get authors
            let authors = item
                .get("author")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| {
                            let given = a.get("given").and_then(|g| g.as_str()).unwrap_or("");
                            let family = a.get("family").and_then(|f| f.as_str()).unwrap_or("");
                            if given.is_empty() && family.is_empty() {
                                None
                            } else {
                                Some(format!("{} {}", given, family))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .map(|s| s.to_string());

            // Get journal name
            let journal = item
                .get("container-title")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|j| j.as_str())
                .map(|j| j.to_string());

            // Get DOI
            let doi = item
                .get("DOI")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get published date
            let published_date = item
                .get("published-print")
                .and_then(|p| p.get("date-parts"))
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|d| d.as_array())
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.as_u64())
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join("-")
                });

            // Get abstract
            let abstract_text = item
                .get("abstract")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get publisher
            let publisher = item
                .get("publisher")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            // Get volume
            let volume = item
                .get("volume")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // Get issue (unused, skipped)
            let _issue = item.get("issue").and_then(|i| i.as_str());

            // Get pages
            let pages = item
                .get("page")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            // Get tags/subject areas
            let subjects = item.get("subject").and_then(|s| s.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

            // Get URL
            let url = item
                .get("URL")
                .and_then(|u| u.as_str())
                .or_else(|| {
                    item.get("link")
                        .and_then(|links| links.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|link| link.get("URL"))
                        .and_then(|u| u.as_str())
                })
                .unwrap_or_default()
                .to_string();

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

            if let Some(doi) = doi {
                result.metadata.tags = Some(vec![format!("DOI: {}", doi)]);
            }

            if let Some(journal) = journal {
                if !journal.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.insert(0, format!("Journal: {}", journal));
                    } else {
                        result.metadata.tags = Some(vec![format!("Journal: {}", journal)]);
                    }
                }
            }

            if let Some(publisher) = publisher {
                if !publisher.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.push(format!("Publisher: {}", publisher));
                    } else {
                        result.metadata.tags = Some(vec![format!("Publisher: {}", publisher)]);
                    }
                }
            }

            if let Some(volume) = volume {
                if !volume.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.push(format!("Volume: {}", volume));
                    } else {
                        result.metadata.tags = Some(vec![format!("Volume: {}", volume)]);
                    }
                }
            }

            if let Some(pages) = pages {
                if !pages.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.push(format!("Pages: {}", pages));
                    } else {
                        result.metadata.tags = Some(vec![format!("Pages: {}", pages)]);
                    }
                }
            }

            if let Some(subjects) = subjects {
                if !subjects.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.extend(subjects);
                    } else {
                        result.metadata.tags = Some(subjects);
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

impl Default for Crossref {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Crossref {
    fn name(&self) -> &str {
        "crossref"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.crossref.org/")
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

        // Build search query
        query_params.insert("query".to_string(), params.query.clone());
        query_params.insert("offset".to_string(), ((params.pageno - 1) * 20).to_string());

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
    fn test_crossref_request() {
        let crossref = Crossref::new();
        let params = RequestParams::new("quantum computing");
        let request = crossref.request(&params).unwrap();

        assert!(request.url.contains("crossref.org"));
        assert!(request.params.contains_key("query"));
    }
}
