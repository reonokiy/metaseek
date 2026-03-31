//! Associated Press (AP) news search engine implementation
//!
//! Provides access to AP news articles covering global events.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// AP news search engine
pub struct ApNews {
    base_url: String,
}

impl ApNews {
    pub fn new() -> Self {
        Self {
            base_url: "https://apnews.com".to_string(),
        }
    }

    /// Parse AP News results from HTML
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for news article items
        let article_selector =
            Selector::parse(r#"div[data-testid="story-card"], article[class*="story"]"#).unwrap();
        let title_selector = Selector::parse(r#"h2, h3, [data-testid="headline"]"#).unwrap();
        let link_selector = Selector::parse(r#"a[href]"#).unwrap();
        let snippet_selector =
            Selector::parse(r#"p[class*="excerpt"], [data-testid="summary"]"#).unwrap();

        let mut position = 1u32;

        for element in document.select(&article_selector) {
            // Get title
            let title = element
                .select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url = element
                .select(&link_selector)
                .find_map(|a| a.value().attr("href"))
                .map(|u| {
                    if u.starts_with("http") {
                        u.to_string()
                    } else if u.starts_with("/") {
                        format!("https://apnews.com{}", u)
                    } else {
                        u.to_string()
                    }
                })
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get snippet/description
            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Get publication date
            let published_date = element
                .select(&Selector::parse(r#"time, [data-testid="datetime"]"#).unwrap())
                .next()
                .and_then(|t| t.value().attr("datetime"))
                .map(|d| d.to_string());

            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::News;

            if let Some(content) = snippet {
                result = result.with_content(content);
            }

            if let Some(date) = published_date {
                result.metadata.published_date = Some(date);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for ApNews {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for ApNews {
    fn name(&self) -> &str {
        "ap_news"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://apnews.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["news", "general"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("query".to_string(), params.query.clone());

        // Pagination
        if params.pageno > 1 {
            query_params.insert("page".to_string(), params.pageno.to_string());
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Add headers to appear as a real browser
        request = request
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate, br");

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
    fn test_ap_news_request() {
        let ap = ApNews::new();
        let params = RequestParams::new("politics");
        let request = ap.request(&params).unwrap();

        assert!(request.url.contains("apnews.com"));
        assert!(request.params.contains_key("query"));
    }
}
