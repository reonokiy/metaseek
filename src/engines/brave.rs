//! Brave search engine implementation

use super::traits::*;
use crate::results::Result;
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Brave web search engine
pub struct Brave {
    base_url: String,
}

impl Brave {
    pub fn new() -> Self {
        Self {
            base_url: "https://search.brave.com/search".to_string(),
        }
    }

    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Brave result selectors - matching reference SearXNG implementation
        // The reference uses: div[contains(@class, 'snippet ')]
        // We also check for fdb class which Brave uses for result cards
        let result_selector =
            Selector::parse(r#"div[class*="snippet fdb"], div[class*="snippet "], div.snippet"#)
                .unwrap();
        // Title is in a div with class containing 'title'
        let title_selector =
            Selector::parse(r#"div[class*="title"], span[class*="title"]"#).unwrap();
        let link_selector = Selector::parse("a").unwrap();
        // Content is in a div with class 'content' (but not 'site-name-content')
        let snippet_selector =
            Selector::parse(r#"div[class*="snippet-content"], div.content, p[class*="snippet"]"#)
                .unwrap();

        let mut position = 1u32;
        let mut seen_urls = std::collections::HashSet::new();

        for element in document.select(&result_selector) {
            // Get URL first - find the main anchor link
            let url = element
                .select(&link_selector)
                .find_map(|a| {
                    let href = a.value().attr("href")?;
                    // Skip internal Brave links and partial URLs (likely ads)
                    if href.starts_with("http") && !href.contains("brave.com") {
                        Some(href.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Skip duplicates
            if seen_urls.contains(&url) {
                continue;
            }
            seen_urls.insert(url.clone());

            // Get title from title div
            let title = element
                .select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            // Get snippet/content
            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            let mut result = Result::new(url, title, self.name().to_string());
            if let Some(content) = snippet {
                result = result.with_content(content);
            }
            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Brave {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Brave {
    fn name(&self) -> &str {
        "brave"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://search.brave.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["general", "web"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn supports_safesearch(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("source".to_string(), "web".to_string());

        // Pagination - Brave uses 'offset' parameter (0-indexed page number)
        if params.pageno > 1 {
            let offset = params.pageno - 1;
            query_params.insert("offset".to_string(), offset.to_string());
        }

        // Time range
        if let Some(ref time_range) = params.time_range {
            let tf = match time_range {
                crate::query::TimeRange::Day => "pd",
                crate::query::TimeRange::Week => "pw",
                crate::query::TimeRange::Month => "pm",
                crate::query::TimeRange::Year => "py",
            };
            query_params.insert("tf".to_string(), tf.to_string());
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Add cookies matching reference SearXNG implementation
        let safesearch_cookie = match params.safesearch {
            2 => "strict",
            1 => "moderate",
            _ => "off",
        };
        request = request
            .cookie("safesearch", safesearch_cookie)
            .cookie("useLocation", "0")
            .cookie("summarizer", "0");

        // Brave prefers gzip, deflate (not brotli)
        request = request.header("Accept-Encoding", "gzip, deflate");

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            // Brave signals throttling with 429 and a body that says so. Let the
            // classifier name it rather than reporting a bare HTTP error.
            response.classify(0, false, self.name())?;
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let results = self.parse_results(&response.text);

        // Brave answers throttled clients with a 200/429 page that carries no
        // result containers but does carry anti-bot markers. Surface that as a
        // rate-limit / refusal instead of an empty (but "successful") search.
        response.classify(results.len(), false, self.name())?;

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brave_request() {
        let brave = Brave::new();
        let params = RequestParams::new("rust programming");
        let request = brave.request(&params).unwrap();

        assert!(request.url.contains("brave.com"));
        assert!(request.params.contains_key("q"));
    }

    fn response_with(status: u16, body: &str) -> EngineResponse {
        EngineResponse {
            status,
            headers: HashMap::new(),
            text: body.to_string(),
            url: "https://search.brave.com/search".to_string(),
        }
    }

    #[test]
    fn test_brave_429_is_reported_as_too_many_requests() {
        let brave = Brave::new();
        let body = r#"<!DOCTYPE html><html><head><title>Brave Search</title></head>
            <body><div class="Blocked">Blocked</div>
            <p>You have sent too many requests. Please slow down.</p></body></html>"#;

        let err = brave.response(response_with(429, body)).unwrap_err();
        assert!(
            err.to_string().contains("TooManyRequests"),
            "throttled response should be reported as rate limited, got: {}",
            err
        );
    }

    #[test]
    fn test_brave_captcha_wall_is_reported_as_captcha() {
        let brave = Brave::new();
        let body = r#"<!DOCTYPE html><html><head><title>Brave Search</title></head>
            <body><div class="Blocked">Blocked</div>
            <p>Please solve the CAPTCHA to continue</p></body></html>"#;

        let err = brave.response(response_with(429, body)).unwrap_err();
        assert!(
            err.to_string().contains("CAPTCHA"),
            "captcha wall should be reported as CAPTCHA, got: {}",
            err
        );
    }

    #[test]
    fn test_brave_200_without_containers_is_a_parse_failure() {
        let brave = Brave::new();
        let body = format!("<!DOCTYPE html><html><body>{}</body></html>", "x".repeat(30_000));

        let err = brave.response(response_with(200, &body)).unwrap_err();
        assert!(
            err.to_string().contains("Failed to parse"),
            "oversized body with no containers should be a parse failure, got: {}",
            err
        );
    }
}
