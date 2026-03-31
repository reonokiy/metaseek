//! Arch Linux package search engine implementation
//!
//! Search for Arch Linux packages in the official repositories and AUR.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Arch Linux package search engine
pub struct ArchLinux {
    base_url: String,
}

impl ArchLinux {
    pub fn new() -> Self {
        Self {
            base_url: "https://archlinux.org/packages/".to_string(),
        }
    }

    /// Parse Arch Linux HTML package results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for package rows
        let result_selector = Selector::parse("table tbody tr").expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get package name/link
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
                .map(|u| format!("https://archlinux.org{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get repository
            let repository = element
                .select(&Selector::parse("td.repo").unwrap())
                .next()
                .and_then(|r| r.text().next())
                .map(|s| s.to_string());

            // Get version
            let version = element
                .select(&Selector::parse("td.version").unwrap())
                .next()
                .and_then(|v| v.text().next())
                .map(|s| s.to_string());

            // Get architecture
            let arch = element
                .select(&Selector::parse("td.arch").unwrap())
                .next()
                .and_then(|a| a.text().next())
                .map(|s| s.to_string());

            // Get description
            let content = element
                .select(&Selector::parse("td.description").unwrap())
                .next()
                .and_then(|d| d.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = content {
                result = result.with_content(desc);
            }

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(repo) = repository {
                result.metadata.tags = Some(vec![repo]);
            }

            if let Some(arch_str) = arch {
                result.metadata.architecture = Some(arch_str);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for ArchLinux {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for ArchLinux {
    fn name(&self) -> &str {
        "archlinux"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://archlinux.org")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "packages", "linux", "aur"]
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
    fn test_arch_request() {
        let arch = ArchLinux::new();
        let params = RequestParams::new("curl");
        let request = arch.request(&params).unwrap();

        assert!(request.url.contains("archlinux.org"));
        assert!(request.params.contains_key("search"));
    }
}
