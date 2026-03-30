//! Void Linux package search engine implementation
//!
//! Search for Void Linux packages in the official repositories.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Void Linux package search engine
pub struct VoidLinux {
    base_url: String,
}

impl VoidLinux {
    pub fn new() -> Self {
        Self {
            base_url: "https://pkgs.qos.xyz/search".to_string(),
        }
    }

    /// Parse Void Linux HTML package results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for package rows
        let result_selector = Selector::parse("div.pkgbox")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get package name/link
            let title_elem = element
                .select(&Selector::parse("a.pkg").unwrap())
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
                .map(|u| format!("https://pkgs.qos.xyz{}", u))
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
                .select(&Selector::parse("span.ver").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get repository
            let repository = element
                .select(&Selector::parse("span.repos").unwrap())
                .next()
                .and_then(|r| r.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(repo) = repository {
                result.metadata.tags = Some(vec![repo]);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for VoidLinux {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for VoidLinux {
    fn name(&self) -> &str {
        "voidlinux"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://pkgs.qos.xyz")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "linux"]
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
    fn test_void_request() {
        let void = VoidLinux::new();
        let params = RequestParams::new("curl");
        let request = void.request(&params).unwrap();

        assert!(request.url.contains("qos.xyz"));
        assert!(request.params.contains_key("q"));
    }
}