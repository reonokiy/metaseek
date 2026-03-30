//! SourceHut package search engine implementation
//!
//! Search for projects on SourceHut.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// SourceHut package search engine
pub struct SourceHut {
    base_url: String,
}

impl SourceHut {
    pub fn new() -> Self {
        Self {
            base_url: "https://sr.ht/projects".to_string(),
        }
    }

    /// Parse SourceHut HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for event results
        let result_selector = Selector::parse("div.event")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title
            let title_elem = element
                .select(&Selector::parse("h4").unwrap())
                .next();

            let title = match title_elem {
                Some(elem) => elem.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            // Get maintainer
            let maintainer = element
                .select(&Selector::parse("h4 a:nth-child(1)").unwrap())
                .next()
                .and_then(|m| m.text().next())
                .map(|s| s.trim_start_matches('~').to_string());

            // Get package name
            let package_name = element
                .select(&Selector::parse("h4 a:nth-child(2)").unwrap())
                .next()
                .and_then(|p| p.text().next())
                .map(|s| s.to_string());

            // Get URL
            let url_elem = element
                .select(&Selector::parse("h4 a:nth-child(2)").unwrap())
                .next();

            let url = url_elem
                .and_then(|e| e.value().attr("href"))
                .map(|u| format!("https://sr.ht/projects{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get description
            let content = element
                .select(&Selector::parse("p").unwrap())
                .next()
                .and_then(|c| c.text().next())
                .map(|s| s.to_string());

            // Get tags
            let tags = element
                .select(&Selector::parse("div.tags a").unwrap())
                .filter_map(|t| t.text().next())
                .map(|s| s.trim_start_matches('#').to_string())
                .collect::<Vec<_>>();

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.template = Some("packages.html".to_string());

            if let Some(pkg_name) = package_name {
                result.metadata.package_name = Some(pkg_name);
            }

            if let Some(maint) = maintainer {
                result.metadata.author = Some(maint);
            }

            if !tags.is_empty() {
                result.metadata.tags = Some(tags);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for SourceHut {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SourceHut {
    fn name(&self) -> &str {
        "sourcehut"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://sr.ht")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos", "code"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), params.query.clone());

        // Sort order (default: recently-updated)
        query_params.insert("sort".to_string(), "recently-updated".to_string());

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
    fn test_sourcehut_request() {
        let sourcehut = SourceHut::new();
        let params = RequestParams::new("rust");
        let request = sourcehut.request(&params).unwrap();

        assert!(request.url.contains("sr.ht"));
        assert!(request.params.contains_key("search"));
    }
}