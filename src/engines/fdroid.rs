//! F-Droid package search engine implementation
//!
//! Search for Android apps on F-Droid.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// F-Droid package search engine
pub struct Fdroid {
    base_url: String,
}

impl Fdroid {
    pub fn new() -> Self {
        Self {
            base_url: "https://search.f-droid.org/".to_string(),
        }
    }

    /// Parse F-Droid HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for app results
        let result_selector =
            Selector::parse("a.package-header").expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/package name
            let title_elem = element
                .select(&Selector::parse("h4.package-name").unwrap())
                .next();

            let title = match title_elem {
                Some(elem) => elem.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url = element
                .value()
                .attr("href")
                .map(|u| format!("https://search.f-droid.org{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get summary
            let content_elem = element
                .select(&Selector::parse("div.package-summary").unwrap())
                .next();

            let content = content_elem
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Get license
            let license_elem = element
                .select(&Selector::parse("div.package-license").unwrap())
                .next();

            let license = license_elem.map(|e| e.text().collect::<String>().trim().to_string());

            // Get thumbnail/icon
            let thumbnail_elem = element
                .select(&Selector::parse("img.package-icon").unwrap())
                .next();

            let thumbnail = thumbnail_elem
                .and_then(|e| e.value().attr("src"))
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.template = Some("packages.html".to_string());

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(thumb) = thumbnail {
                result.metadata.thumbnail = Some(thumb);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Fdroid {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Fdroid {
    fn name(&self) -> &str {
        "fdroid"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://f-droid.org")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["files", "apps", "android"]
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
    fn test_fdroid_request() {
        let fdroid = Fdroid::new();
        let params = RequestParams::new("firefox");
        let request = fdroid.request(&params).unwrap();

        assert!(request.url.contains("f-droid.org"));
        assert!(request.params.contains_key("q"));
    }
}
