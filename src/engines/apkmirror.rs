//! APKMirror search engine implementation
//!
//! Search for Android APK files on APKMirror.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// APKMirror search engine
pub struct ApkMirror {
    base_url: String,
}

impl ApkMirror {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.apkmirror.com/".to_string(),
        }
    }

    /// Parse APKMirror HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for app rows
        let result_selector = Selector::parse("div.appRow").expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title/app name
            let title_elem = element.select(&Selector::parse("h5 a").unwrap()).next();

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
                .map(|u| format!("https://www.apkmirror.com{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get thumbnail
            let thumbnail_elem = element.select(&Selector::parse("img").unwrap()).next();

            let thumbnail = thumbnail_elem
                .and_then(|e| e.value().attr("src"))
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            result.metadata.template = Some("packages.html".to_string());

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

impl Default for ApkMirror {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for ApkMirror {
    fn name(&self) -> &str {
        "apkmirror"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.apkmirror.com")
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
        let query = "post_type=app_release&searchtype=apk".to_string();
        let mut query_params = HashMap::new();
        query_params.insert("s".to_string(), params.query.clone());

        // Build query string manually
        let parts: Vec<String> = query_params
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    urlencoding::encode(k.as_str()),
                    urlencoding::encode(v.as_str())
                )
            })
            .collect();
        let query_string = format!("&{}", parts.join("&"));
        let url = format!("{}?{}{}", self.base_url, query, query_string);

        let mut request = EngineRequest::get(&url);
        request.params = HashMap::new();

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
    fn test_apkmirror_request() {
        let apk = ApkMirror::new();
        let params = RequestParams::new("chrome");
        let request = apk.request(&params).unwrap();

        assert!(request.url.contains("apkmirror.com"));
    }
}
