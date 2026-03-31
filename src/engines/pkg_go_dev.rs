//! pkg.go.dev search engine implementation
//!
//! Search for Go packages on pkg.go.dev.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// pkg.go.dev package search engine
pub struct PkgGoDev {
    base_url: String,
}

impl PkgGoDev {
    pub fn new() -> Self {
        Self {
            base_url: "https://pkg.go.dev/search".to_string(),
        }
    }

    /// Parse pkg.go.dev HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for search results
        let result_selector =
            Selector::parse("div.SearchSnippet").expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/link
            let title_elem = element.select(&Selector::parse("h2 a").unwrap()).next();

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
                .map(|u| format!("https://pkg.go.dev{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get package name
            let package_name = element
                .select(&Selector::parse("span.SearchSnippet-headerContainer span").unwrap())
                .next()
                .and_then(|p| p.text().next())
                .map(|s| s.to_string());

            // Get version
            let version = element
                .select(&Selector::parse("div.SearchSnippet-infoLabel span strong").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get popularity
            let popularity = element
                .select(&Selector::parse("div.SearchSnippet-infoLabel a strong").unwrap())
                .next()
                .and_then(|p| p.text().next())
                .map(|s| s.to_string());

            // Get license
            let license = element
                .select(&Selector::parse("div.SearchSnippet-infoLabel span[title]").unwrap())
                .next()
                .and_then(|l| l.text().next())
                .map(|s| s.to_string());

            // Get description
            let content = element
                .select(&Selector::parse("p.SearchSnippet-synopsis").unwrap())
                .next()
                .and_then(|c| c.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = content {
                result = result.with_content(desc);
            }

            if let Some(pkg_name) = package_name {
                result.metadata.package_name = Some(pkg_name);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(pop_str) = popularity {
                // Parse popularity number (may contain K, M, B suffixes)
                if let Some(pop) = Self::parse_popularity(&pop_str) {
                    result.metadata.views = Some(pop);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }

    /// Parse popularity string to u64
    fn parse_popularity(popularity: &str) -> Option<u64> {
        let popularity = popularity.trim();

        // Handle European number format (e.g., "15.000,00" -> 15000)
        if popularity.contains('.') && popularity.contains(',') {
            // European format: 15.000,00
            let cleaned = popularity.replace('.', "").replace(',', ".");
            return cleaned.parse::<f64>().ok().map(|v| v as u64);
        }

        if popularity.contains("K") || popularity.contains("k") {
            popularity
                .trim_end_matches(['K', 'k'])
                .replace(',', "")
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1000.0) as u64)
        } else if popularity.contains("M") || popularity.contains("m") {
            popularity
                .trim_end_matches(['M', 'm'])
                .replace(',', "")
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1_000_000.0) as u64)
        } else if popularity.contains("B") || popularity.contains("b") {
            popularity
                .trim_end_matches(['B', 'b'])
                .replace(',', "")
                .parse::<f64>()
                .ok()
                .map(|v| (v * 1_000_000_000.0) as u64)
        } else {
            popularity.replace(',', "").parse().ok()
        }
    }
}

impl Default for PkgGoDev {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for PkgGoDev {
    fn name(&self) -> &str {
        "pkg_go_dev"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://pkg.go.dev")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "go"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("m".to_string(), "package".to_string());

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
    fn test_pkg_go_dev_request() {
        let pkg = PkgGoDev::new();
        let params = RequestParams::new("github.com");
        let request = pkg.request(&params).unwrap();

        assert!(request.url.contains("pkg.go.dev"));
        assert!(request.params.contains_key("q"));
    }

    #[test]
    fn test_parse_popularity() {
        assert_eq!(PkgGoDev::parse_popularity("15.000,00"), Some(15000));
        assert_eq!(PkgGoDev::parse_popularity("1.2k"), Some(1200));
        assert_eq!(PkgGoDev::parse_popularity("10M"), Some(10_000_000));
    }
}
