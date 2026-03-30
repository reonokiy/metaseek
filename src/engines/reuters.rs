//! Reuters news search engine implementation
//!
//! Provides access to Reuters news articles and financial information.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Reuters news search engine
pub struct Reuters {
    base_url: String,
}

impl Reuters {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.reuters.com".to_string(),
        }
    }

    /// Parse Reuters news results from HTML
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for news article cards
        let article_selector =
            Selector::parse(r#"div[data-testid="articleCard"], article[data-testid="articleCard"]"#)
                .unwrap();
        let title_selector = Selector::parse(r#"h2, [data-testid="headline"]"#).unwrap();
        let link_selector = Selector::parse(r#"a[href]"#).unwrap();
        let snippet_selector = Selector::parse(r#"p[data-testid="excerpt"], div[data-testid="description"]"#)
            .unwrap();
        let timestamp_selector = Selector::parse(r#"time, [data-testid="timestamp"]"#).unwrap();

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
                    } else {
                        format!("https://www.reuters.com{}", u)
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
                .select(&timestamp_selector)
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

impl Default for Reuters {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Reuters {
    fn name(&self) -> &str {
        "reuters"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.reuters.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["news", "financial", "general"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("type".to_string(), "Articles".to_string());

        // Pagination
        if params.pageno > 1 {
            query_params.insert("p".to_string(), ((params.pageno - 1) * 10).to_string());
        }

        // Time range
        if let Some(ref time_range) = params.time_range {
            let date_param = match time_range {
                crate::query::TimeRange::Day => "d",
                crate::query::TimeRange::Week => "w",
                crate::query::TimeRange::Month => "m",
                crate::query::TimeRange::Year => "y",
            };
            query_params.insert("dateRange".to_string(), date_param.to_string());
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Add headers to appear as a real browser
        request = request
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header(
                "Accept-Language",
                "en-US,en;q=0.5,text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1");

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
    fn test_reuters_request() {
        let reuters = Reuters::new();
        let params = RequestParams::new("technology news");
        let request = reuters.request(&params).unwrap();

        assert!(request.url.contains("reuters.com"));
        assert!(request.params.contains_key("q"));
    }
}