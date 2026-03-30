//! lib.rs search engine implementation
//!
//! Search for Rust packages on lib.rs, a browsable interface for crates.io.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// lib.rs package search engine
pub struct LibRs {
    base_url: String,
}

impl LibRs {
    pub fn new() -> Self {
        Self {
            base_url: "https://lib.rs".to_string(),
        }
    }

    /// Parse lib.rs HTML response
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for search results
        let result_selector = Selector::parse("li.search-result")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/link
            let title_elem = element
                .select(&Selector::parse("h4 a").unwrap())
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
                .map(|u| format!("https://lib.rs{}", u))
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
                .select(&Selector::parse("span.version").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get downloads
            let downloads = element
                .select(&Selector::parse("span.downloads").unwrap())
                .next()
                .and_then(|d| d.text().next())
                .map(|s| s.to_string());

            // Get stars
            let stars = element
                .select(&Selector::parse("span.stars").unwrap())
                .next()
                .and_then(|s| s.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.version = version;
            result.metadata.views = downloads.and_then(|s| s.parse().ok());
            result.metadata.template = Some("packages.html".to_string());

            if let Some(stars_str) = stars {
                if let Ok(stars) = stars_str.parse::<u64>() {
                    result.metadata.stars = Some(stars);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for LibRs {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for LibRs {
    fn name(&self) -> &str {
        "lib_rs"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://lib.rs")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "rust"]
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
    fn test_lib_rs_request() {
        let lib_rs = LibRs::new();
        let params = RequestParams::new("regex");
        let request = lib_rs.request(&params).unwrap();

        assert!(request.url.contains("lib.rs"));
        assert!(request.params.contains_key("q"));
    }
}