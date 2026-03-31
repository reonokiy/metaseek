//! Alpine Linux package search engine implementation
//!
//! Search for Alpine Linux packages in the official repositories.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Alpine Linux package search engine
pub struct AlpineLinux {
    base_url: String,
}

impl AlpineLinux {
    pub fn new() -> Self {
        Self {
            base_url: "https://pkgs.alpinelinux.org/packages".to_string(),
        }
    }

    /// Parse Alpine Linux HTML package results
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
                .map(|u| format!("https://pkgs.alpinelinux.org{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

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

            // Get repository
            let repository = element
                .select(&Selector::parse("td.repository").unwrap())
                .next()
                .and_then(|r| r.text().next())
                .map(|s| s.to_string());

            // Get license
            let license = element
                .select(&Selector::parse("td.license").unwrap())
                .next()
                .and_then(|l| l.text().next())
                .map(|s| s.to_string());

            // Get maintainer
            let maintainer = element
                .select(&Selector::parse("td.maintainer").unwrap())
                .next()
                .and_then(|m| m.text().next())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            result.metadata.version = version;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(repo) = repository {
                result.metadata.tags = Some(vec![repo]);
            }

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(maint) = maintainer {
                result.metadata.author = Some(maint);
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

impl Default for AlpineLinux {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for AlpineLinux {
    fn name(&self) -> &str {
        "alpinelinux"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://pkgs.alpinelinux.org")
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
        query_params.insert("name".to_string(), format!("*{}*", params.query));

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
    fn test_alpine_request() {
        let alpine = AlpineLinux::new();
        let params = RequestParams::new("curl");
        let request = alpine.request(&params).unwrap();

        assert!(request.url.contains("alpinelinux.org"));
        assert!(request.params.contains_key("name"));
    }
}
