//! DuckDuckGo search engine implementation

use super::traits::*;
use crate::results::{Answer, Result, Suggestion};
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// DuckDuckGo web search engine
pub struct DuckDuckGo {
    _base_url: String,
    html_url: String,
}

impl DuckDuckGo {
    pub fn new() -> Self {
        Self {
            _base_url: "https://api.duckduckgo.com/".to_string(),
            html_url: "https://html.duckduckgo.com/html/".to_string(),
        }
    }

    fn parse_html_results(&self, html: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // DuckDuckGo HTML result selectors - matching reference SearXNG implementation
        // Reference uses: //div[@id="links"]/div[contains(@class, "web-result")]
        let links_selector = Selector::parse("#links").unwrap();
        let result_selector = Selector::parse(r#"div[class*="web-result"], div.result"#).unwrap();
        // Title is in h2/a
        let title_link_selector = Selector::parse("h2 a, a.result__a").unwrap();
        // Content/snippet
        let snippet_selector =
            Selector::parse(r#"a[class*="result__snippet"], a.result__snippet"#).unwrap();

        let mut position = 1u32;

        // First try to find the #links container
        let search_area = document
            .select(&links_selector)
            .next()
            .map(|e| e.html())
            .unwrap_or_else(|| document.html());

        let search_doc = Html::parse_document(&search_area);

        for element in search_doc.select(&result_selector) {
            // Skip ad results
            let class_attr = element.value().attr("class").unwrap_or("");
            if class_attr.contains("result--ad") {
                continue;
            }

            // Get title and URL from h2/a
            let title_elem = match element.select(&title_link_selector).next() {
                Some(t) => t,
                None => continue,
            };

            let title = title_elem.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }

            let url = title_elem
                .value()
                .attr("href")
                .map(|h| h.to_string())
                .unwrap_or_default();

            // Skip DuckDuckGo internal links and empty URLs
            if url.is_empty() || url.contains("duckduckgo.com") || !url.starts_with("http") {
                continue;
            }

            // Get snippet
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

impl Default for DuckDuckGo {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for DuckDuckGo {
    fn name(&self) -> &str {
        "duckduckgo"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://duckduckgo.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["general", "web"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_safesearch(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut form_data = HashMap::new();
        form_data.insert("q".to_string(), params.query.clone());

        // First page uses empty 'b', subsequent pages need pagination params
        if params.pageno == 1 {
            form_data.insert("b".to_string(), String::new());
        } else {
            // Page 2 = 10, Page 3+ = 10 + (n-2)*15
            let offset = 10 + (params.pageno.saturating_sub(2) as i32 * 15);
            form_data.insert("s".to_string(), offset.to_string());
            form_data.insert("dc".to_string(), (offset + 1).to_string());
            form_data.insert("v".to_string(), "l".to_string());
            form_data.insert("o".to_string(), "json".to_string());
            form_data.insert("api".to_string(), "d.js".to_string());
        }

        // Region/language - use empty for 'all'
        let kl = if params.lang == "all" || params.lang.is_empty() {
            String::new()
        } else {
            params.lang.clone()
        };
        form_data.insert("kl".to_string(), kl);

        // Safe search
        let kp = match params.safesearch {
            2 => "1",  // Strict
            1 => "-1", // Moderate
            _ => "-2", // Off
        };
        form_data.insert("kp".to_string(), kp.to_string());

        let mut request = EngineRequest::post(&self.html_url).form(form_data);

        // Add headers matching reference SearXNG implementation
        // These are important for DDG's bot detection
        request = request
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Referer", &self.html_url)
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-User", "?1");

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            // DDG's challenge/anomaly pages carry their own explanatory body;
            // classify them so the refusal reason is not lost.
            response.classify(0, false, self.name())?;
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let results = self.parse_html_results(&response.text);

        // When DDG refuses a client it still answers 200 with its challenge /
        // no-results shell. Zero parsed results from a body that contains no
        // `web-result` container at all is a refusal, not an empty SERP.
        response.classify(results.len(), false, self.name())?;

        Ok(EngineResults::with_results(results))
    }
}

/// DuckDuckGo Instant Answer API (for answers and suggestions)
pub struct DuckDuckGoInstant {
    api_url: String,
}

impl DuckDuckGoInstant {
    pub fn new() -> Self {
        Self {
            api_url: "https://api.duckduckgo.com/".to_string(),
        }
    }
}

impl Default for DuckDuckGoInstant {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for DuckDuckGoInstant {
    fn name(&self) -> &str {
        "duckduckgo_instant"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://duckduckgo.com")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["general"]
    }

    fn supports_paging(&self) -> bool {
        false
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("format".to_string(), "json".to_string());
        query_params.insert("no_redirect".to_string(), "1".to_string());
        query_params.insert("no_html".to_string(), "1".to_string());

        let mut request = EngineRequest::get(&self.api_url);
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let json: serde_json::Value = serde_json::from_str(&response.text)?;
        let mut engine_results = EngineResults::new();

        // Abstract (instant answer)
        if let Some(abstract_text) = json.get("AbstractText").and_then(|v| v.as_str()) {
            if !abstract_text.is_empty() {
                let answer = Answer::new(abstract_text.to_string(), self.name().to_string());
                engine_results.add_answer(answer);
            }
        }

        // Related topics as suggestions
        if let Some(related) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
            for topic in related.iter().take(5) {
                if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                    let suggestion = Suggestion {
                        text: text.to_string(),
                        engine: self.name().to_string(),
                    };
                    engine_results.add_suggestion(suggestion);
                }
            }
        }

        // Results from topics
        if let Some(results) = json.get("Results").and_then(|v| v.as_array()) {
            for (i, result) in results.iter().enumerate() {
                if let (Some(url), Some(text)) = (
                    result.get("FirstURL").and_then(|v| v.as_str()),
                    result.get("Text").and_then(|v| v.as_str()),
                ) {
                    let mut r =
                        Result::new(url.to_string(), text.to_string(), self.name().to_string());
                    r = r.with_position((i + 1) as u32);
                    engine_results.add_result(r);
                }
            }
        }

        Ok(engine_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duckduckgo_request() {
        let ddg = DuckDuckGo::new();
        let params = RequestParams::new("rust programming");
        let request = ddg.request(&params).unwrap();

        assert!(request.url.contains("duckduckgo.com"));
    }

    fn html_response(status: u16, body: &str) -> EngineResponse {
        EngineResponse {
            status,
            headers: HashMap::new(),
            text: body.to_string(),
            url: "https://html.duckduckgo.com/html/".to_string(),
        }
    }

    #[test]
    fn test_ddg_non_html_body_is_a_parse_failure() {
        let ddg = DuckDuckGo::new();
        let err = ddg
            .response(html_response(200, "upstream temporarily unavailable"))
            .unwrap_err();
        assert!(
            err.to_string().contains("Failed to parse"),
            "non-HTML body should be a parse failure, got: {}",
            err
        );
    }

    #[test]
    fn test_ddg_small_empty_serp_is_not_an_error() {
        let ddg = DuckDuckGo::new();
        let body = r#"<!DOCTYPE html><html><body><div id="links"></div>
            <p>No results.</p></body></html>"#;

        let results = ddg.response(html_response(200, body)).unwrap();
        assert!(results.results.is_empty());
    }
}
