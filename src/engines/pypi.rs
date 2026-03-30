//! PyPI (Python Package Index) search engine implementation
//!
//! Search for Python packages on PyPI.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// PyPI package search engine
pub struct PyPi {
    base_url: String,
}

impl PyPi {
    pub fn new() -> Self {
        Self {
            base_url: "https://pypi.org/search/".to_string(),
        }
    }

    /// Parse PyPI HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for search results
        let result_selector = Selector::parse("div.js-search-results .package-snippet")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/link
            let title_elem = element
                .select(&Selector::parse("a").unwrap())
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
                .map(|u| format!("https://pypi.org{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get description
            let content = element
                .select(&Selector::parse("p").unwrap())
                .next()
                .map(|p| p.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Get version
            let version = element
                .select(&Selector::parse("span.package-snippet__version").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get author
            let author = element
                .select(&Selector::parse("span.package-snippet__author").unwrap())
                .next()
                .and_then(|a| a.text().next())
                .map(|s| s.to_string());

            // Get year
            let year = element
                .select(&Selector::parse("span.package-snippet__year").unwrap())
                .next()
                .and_then(|y| y.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.version = version;
            result.metadata.author = author;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(y) = year {
                if let Ok(year_num) = y.parse::<u32>() {
                    result.metadata.published_date = Some(year_num.to_string());
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for PyPi {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for PyPi {
    fn name(&self) -> &str {
        "pypi"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://pypi.org")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "python"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());

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
    fn test_pypi_request() {
        let pypi = PyPi::new();
        let params = RequestParams::new("requests");
        let request = pypi.request(&params).unwrap();

        assert!(request.url.contains("pypi.org"));
        assert!(request.params.contains_key("q"));
    }
}