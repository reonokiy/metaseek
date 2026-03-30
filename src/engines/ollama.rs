//! Ollama model search engine implementation
//!
//! Search for AI/ML models on Ollama.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Ollama model search engine
pub struct Ollama {
    base_url: String,
}

impl Ollama {
    pub fn new() -> Self {
        Self {
            base_url: "https://ollama.com/search".to_string(),
        }
    }

    /// Parse Ollama HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for model results
        let result_selector = Selector::parse("li[x-test-model]")
            .expect("Failed to parse selector");

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            // Get title
            let title_elem = element
                .select(&Selector::parse("span[x-test-search-response-title]").unwrap())
                .next();

            let title = match title_elem {
                Some(elem) => elem.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url_elem = element
                .select(&Selector::parse("a").unwrap())
                .next();

            let url = url_elem
                .and_then(|e| e.value().attr("href"))
                .map(|u| format!("https://ollama.com{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get description
            let content = element
                .select(&Selector::parse("p.max-w-lg.break-words.text-neutral-800.text-md").unwrap())
                .next()
                .and_then(|c| c.text().next())
                .map(|s| s.to_string());

            // Get published date
            let published_date = element
                .select(&Selector::parse("span.flex.items-center").unwrap())
                .next()
                .and_then(|d| d.value().attr("title"))
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            result.metadata.template = Some("packages.html".to_string());

            if let Some(pub_date) = published_date {
                result.metadata.published_date = Some(pub_date);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Ollama {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Ollama {
    fn name(&self) -> &str {
        "ollama"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://ollama.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos", "ai", "ml"]
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
    fn test_ollama_request() {
        let ollama = Ollama::new();
        let params = RequestParams::new("llama3");
        let request = ollama.request(&params).unwrap();

        assert!(request.url.contains("ollama.com"));
        assert!(request.params.contains_key("q"));
    }
}