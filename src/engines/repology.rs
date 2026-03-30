//! Repology package search engine implementation
//!
//! Repology aggregates package information from various distributions and repositories.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Repology package search engine
pub struct Repology {
    base_url: String,
}

impl Repology {
    pub fn new() -> Self {
        Self {
            base_url: "https://repology.org/search".to_string(),
        }
    }

    /// Parse Repology HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for search results
        let result_selector = Selector::parse("tr.packages-list__row")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/link
            let title_elem = element
                .select(&Selector::parse("a.packages-list__name").unwrap())
                .next();

            let title = match title_elem {
                Some(elem) => elem.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url = title_elem
                .and_then(|e| e.value().attr("href"))
                .map(|u| format!("https://repology.org{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get version
            let version = element
                .select(&Selector::parse("td.packages-list__version").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get repository
            let repository = element
                .select(&Selector::parse("td.packages-list__repo").unwrap())
                .next()
                .and_then(|r| r.text().next())
                .map(|s| s.to_string());

            // Get category
            let category = element
                .select(&Selector::parse("td.packages-list__category").unwrap())
                .next()
                .and_then(|c| c.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(repo) = repository {
                result.metadata.tags = Some(vec![repo]);
            }

            if let Some(cat) = category {
                result.category = Some(cat);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Repology {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Repology {
    fn name(&self) -> &str {
        "repology"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://repology.org")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), params.query.clone());

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let results = self.parse_results(&response.text);

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repology_request() {
        let repology = Repology::new();
        let params = RequestParams::new("curl");
        let request = repology.request(&params).unwrap();

        assert!(request.url.contains("repology.org"));
        assert!(request.params.contains_key("search"));
    }
}