//! Fortune 500 corporate search engine implementation
//!
//! Provides access to Fortune 500 company information and rankings.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Fortune 500 corporate search engine
pub struct Fortune500;

impl Fortune500 {
    pub fn new() -> Self {
        Self
    }

    /// Parse Fortune 500 results from HTML
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Selector for company cards
        let company_selector =
            Selector::parse(r#"div[data-testid="company-card"], .company-card"#).unwrap();
        let name_selector = Selector::parse(r#"h2, [data-testid="company-name"]"#).unwrap();
        let link_selector = Selector::parse(r#"a[href*="/company/"]"#).unwrap();
        let description_selector =
            Selector::parse(r#"p[class*="description"], [data-testid="company-description"]"#)
                .unwrap();
        let rank_selector = Selector::parse(r#"[data-testid="rank"], .rank"#).unwrap();

        let mut position = 1u32;

        for element in document.select(&company_selector) {
            // Get company name
            let name = element
                .select(&name_selector)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() {
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
                        format!("https://fortune.com{}", u)
                    } else {
                        u.to_string()
                    }
                })
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get snippet/description
            let description = element
                .select(&description_selector)
                .next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Get rank
            let rank = element
                .select(&rank_selector)
                .next()
                .map(|r| r.text().collect::<String>().trim().to_string());

            let mut result = Result::new(url, name, self.name().to_string());
            result.result_type = ResultType::Corporate;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            if let Some(rank_str) = rank {
                result.metadata.tags = Some(vec![format!("Rank: {}", rank_str)]);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Fortune500 {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Fortune500 {
    fn name(&self) -> &str {
        "fortune500"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://fortune.com/ranking/fortune500")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["corporate", "business", "financial"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        // Fortune 500 doesn't support direct search, we'll search Fortune's corporate section
        let search_url = "https://fortune.com/search/?q=".to_string();

        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());

        // Pagination
        if params.pageno > 1 {
            query_params.insert("page".to_string(), params.pageno.to_string());
        }

        let mut request = EngineRequest::get(&search_url);
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
    fn test_fortune500_request() {
        let fortune = Fortune500::new();
        let params = RequestParams::new("technology companies");
        let request = fortune.request(&params).unwrap();

        assert!(request.url.contains("fortune.com"));
        assert!(request.params.contains_key("q"));
    }
}
