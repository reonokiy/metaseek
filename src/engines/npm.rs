//! NPM package search engine implementation
//!
//! Search for JavaScript/Node.js packages on npm.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// NPM package search engine
pub struct Npm {
    base_url: String,
}

impl Npm {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.npmjs.com/search".to_string(),
        }
    }

    /// Parse NPM HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for search results
        let result_selector = Selector::parse("div[data-automation='package-result']")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/link
            let title_elem = element.select(&Selector::parse("a").unwrap()).next();

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
                .map(|u| format!("https://www.npmjs.com{}", u))
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
                .select(&Selector::parse("span[data-automation='package-result-version']").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get downloads
            let downloads = element
                .select(
                    &Selector::parse("span[data-automation='package-result-downloads']").unwrap(),
                )
                .next()
                .and_then(|d| d.text().next())
                .map(|s| s.to_string());

            // Get last updated
            let last_updated = element
                .select(
                    &Selector::parse("span[data-automation='package-result-last-updated']")
                        .unwrap(),
                )
                .next()
                .and_then(|d| d.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(downloads_str) = downloads {
                // Extract number from downloads string (e.g., "1.2k", "10M")
                let parsed_downloads = Self::parse_downloads(&downloads_str);
                if let Some(count) = parsed_downloads {
                    result.metadata.views = Some(count);
                }
            }

            if let Some(updated) = last_updated {
                result.metadata.last_update = Some(updated);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }

    /// Parse downloads string to u64
    fn parse_downloads(downloads: &str) -> Option<u64> {
        let downloads = downloads.trim();

        if downloads.contains("k") || downloads.contains("K") {
            downloads
                .trim_end_matches(['k', 'K'])
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1000.0) as u64)
        } else if downloads.contains("m") || downloads.contains("M") {
            downloads
                .trim_end_matches(['m', 'M'])
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1_000_000.0) as u64)
        } else if downloads.contains("b") || downloads.contains("B") {
            downloads
                .trim_end_matches(['b', 'B'])
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1_000_000_000.0) as u64)
        } else {
            downloads.parse().ok()
        }
    }
}

impl Default for Npm {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Npm {
    fn name(&self) -> &str {
        "npm"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.npmjs.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "javascript", "nodejs"]
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
    fn test_npm_request() {
        let npm = Npm::new();
        let params = RequestParams::new("express");
        let request = npm.request(&params).unwrap();

        assert!(request.url.contains("npmjs.com"));
        assert!(request.params.contains_key("search"));
    }

    #[test]
    fn test_parse_downloads() {
        assert_eq!(Npm::parse_downloads("1.2k"), Some(1200));
        assert_eq!(Npm::parse_downloads("10M"), Some(10_000_000));
        assert_eq!(Npm::parse_downloads("1B"), Some(1_000_000_000));
        assert_eq!(Npm::parse_downloads("1234"), Some(1234));
    }
}
