//! Google search engine implementation

use super::traits::*;
use crate::results::Result;
use anyhow::Result as AnyhowResult;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Google web search engine
pub struct Google {
    base_url: String,
}

impl Google {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.google.com/search".to_string(),
        }
    }

    fn parse_results(&self, html: &str, engine_name: &str) -> Vec<Result> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // Primary selector: Google's modern result container with jscontroller attribute
        // This matches the reference SearXNG Python implementation
        let result_selector =
            Selector::parse(r#"div[jscontroller*="SC7lYd"], div.g, div[data-hveid] > div"#)
                .unwrap();
        let title_selector = Selector::parse("h3").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        // Modern Google uses data-sncf attribute for content containers
        let snippet_selector =
            Selector::parse(r#"div[data-sncf*="1"], div.VwiC3b, span.aCOpRe, div[data-snf]"#)
                .unwrap();

        let mut position = 1u32;
        let mut seen_urls = std::collections::HashSet::new();

        for element in document.select(&result_selector) {
            // Get title - look for h3 inside an anchor first (modern structure)
            let title = element
                .select(&link_selector)
                .find_map(|a| {
                    a.select(&title_selector)
                        .next()
                        .map(|t| t.text().collect::<String>())
                })
                .or_else(|| {
                    // Fallback: direct h3 child
                    element
                        .select(&title_selector)
                        .next()
                        .map(|t| t.text().collect::<String>())
                })
                .unwrap_or_default()
                .trim()
                .to_string();

            if title.is_empty() {
                continue;
            }

            // Get URL - find anchor that contains or precedes the h3
            let url = element
                .select(&link_selector)
                .find(|a| {
                    // Prefer anchor that contains h3, or has valid href
                    a.select(&title_selector).next().is_some()
                        || a.value()
                            .attr("href")
                            .map(|h| h.starts_with("http"))
                            .unwrap_or(false)
                })
                .and_then(|a| a.value().attr("href"))
                .or_else(|| {
                    // Fallback: first anchor with http href
                    element
                        .select(&link_selector)
                        .find_map(|a| a.value().attr("href").filter(|h| h.starts_with("http")))
                })
                .map(|h| h.to_string())
                .unwrap_or_default();

            if url.is_empty() || url.starts_with('/') || url.starts_with('#') {
                continue;
            }

            // Skip duplicates within this parse
            if seen_urls.contains(&url) {
                continue;
            }
            seen_urls.insert(url.clone());

            // Get snippet from content container
            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|s| s.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            let mut result = Result::new(url, title, engine_name.to_string());
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

impl Default for Google {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Google {
    fn name(&self) -> &str {
        "google"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.google.com")
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
        query_params.insert("hl".to_string(), params.lang.clone());
        query_params.insert("ie".to_string(), "utf8".to_string());
        query_params.insert("oe".to_string(), "utf8".to_string());

        // Pagination
        if params.pageno > 1 {
            let start = (params.pageno - 1) * 10;
            query_params.insert("start".to_string(), start.to_string());
        }

        // Safe search
        match params.safesearch {
            2 => {
                query_params.insert("safe".to_string(), "high".to_string());
            }
            1 => {
                query_params.insert("safe".to_string(), "medium".to_string());
            }
            _ => {
                query_params.insert("safe".to_string(), "off".to_string());
            }
        }

        // Time range
        if let Some(ref time_range) = params.time_range {
            let tbs = match time_range {
                crate::query::TimeRange::Day => "qdr:d",
                crate::query::TimeRange::Week => "qdr:w",
                crate::query::TimeRange::Month => "qdr:m",
                crate::query::TimeRange::Year => "qdr:y",
            };
            query_params.insert("tbs".to_string(), tbs.to_string());
        }

        // Disable filtering to get more comprehensive results
        query_params.insert("filter".to_string(), "0".to_string());

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Add CONSENT cookie to bypass consent screen (matches SearXNG Python)
        request = request.cookie("CONSENT", "YES+");

        // Add Accept header
        request = request.header("Accept", "*/*");

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        if response.is_captcha() {
            return Err(anyhow::anyhow!("CAPTCHA detected"));
        }

        // Google serves a JS bootstrap/interstitial page (HTTP 200, no CAPTCHA
        // text) to clients it will not render results for. Report it instead of
        // returning an empty result set that looks like a successful search.
        if response.is_js_gated() {
            return Err(anyhow::anyhow!(
                "Blocked: Google returned a JavaScript-gated interstitial \
                 (datacenter/cloud IP ranges are commonly refused server-rendered results)"
            ));
        }

        let results = self.parse_results(&response.text, self.name());

        // A response this large containing zero matchable result containers means
        // the HTML layout no longer matches our selectors, not that the query had
        // no matches. Surface it so the engine is reported as unresponsive.
        if response.looks_unparsed(results.len()) {
            return Err(anyhow::anyhow!(
                "Failed to parse response: {} bytes of HTML matched no result container",
                response.text.len()
            ));
        }

        Ok(EngineResults::with_results(results))
    }
}

/// Google Images search engine
/// Uses Google's internal JSON API for better results
pub struct GoogleImages {
    base_url: String,
}

impl GoogleImages {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.google.com/search".to_string(),
        }
    }
}

impl Default for GoogleImages {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GoogleImages {
    fn name(&self) -> &str {
        "google_images"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://images.google.com")
            .official_api(false)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["images"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_safesearch(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("tbm".to_string(), "isch".to_string());
        query_params.insert("hl".to_string(), params.lang.clone());
        query_params.insert("asearch".to_string(), "isch".to_string());

        // Use JSON format with pagination (0-indexed)
        let page_index = params.pageno.saturating_sub(1);
        query_params.insert(
            "async".to_string(),
            format!("_fmt:json,p:1,ijn:{}", page_index),
        );

        // Safe search
        if params.safesearch >= 1 {
            query_params.insert("safe".to_string(), "active".to_string());
        }

        // Time range
        if let Some(ref time_range) = params.time_range {
            let tbs = match time_range {
                crate::query::TimeRange::Day => "qdr:d",
                crate::query::TimeRange::Week => "qdr:w",
                crate::query::TimeRange::Month => "qdr:m",
                crate::query::TimeRange::Year => "qdr:y",
            };
            query_params.insert("tbs".to_string(), tbs.to_string());
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        // Use Android app user agent for better results
        request.headers.insert(
            "User-Agent".to_string(),
            "NSTN/3.60.474802233.release Dalvik/2.1.0 (Linux; U; Android 12; US) gzip".to_string(),
        );

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        if response.is_captcha() {
            return Err(anyhow::anyhow!("CAPTCHA detected"));
        }

        // Find the JSON data starting with {"ischj":
        let json_start = response.text.find("{\"ischj\":");
        if json_start.is_none() {
            // Fallback: try to find any JSON object
            return Ok(EngineResults::with_results(vec![]));
        }

        let json_text = &response.text[json_start.unwrap()..];
        let json_data: serde_json::Value = serde_json::from_str(json_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let mut results = Vec::new();
        let mut position = 1u32;

        // Parse the image metadata
        if let Some(metadata) = json_data
            .get("ischj")
            .and_then(|v| v.get("metadata"))
            .and_then(|v| v.as_array())
        {
            for item in metadata {
                // Get the result info
                let result_info = match item.get("result") {
                    Some(r) => r,
                    None => continue,
                };

                // Get URL
                let url = result_info
                    .get("referrer_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                if url.is_empty() {
                    continue;
                }

                // Get title
                let title = result_info
                    .get("page_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                // Get content/snippet
                let content = item
                    .get("text_in_grid")
                    .and_then(|v| v.get("snippet"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get image source
                let img_src = item
                    .get("original_image")
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get thumbnail
                let thumbnail = item
                    .get("thumbnail")
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Get resolution
                let resolution = item.get("original_image").and_then(|img| {
                    let width = img.get("width").and_then(|v| v.as_u64());
                    let height = img.get("height").and_then(|v| v.as_u64());
                    match (width, height) {
                        (Some(w), Some(h)) => Some(format!("{} x {}", w, h)),
                        _ => None,
                    }
                });

                // Get source site
                let source = result_info
                    .get("site_title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Build result
                let mut result = Result::new(url, title, self.name().to_string());
                result.result_type = crate::results::ResultType::Image;
                result = result.with_position(position);

                if let Some(c) = content {
                    result = result.with_content(c);
                }

                result.metadata.img_src = img_src;
                result.metadata.thumbnail = thumbnail;
                result.metadata.template = Some("images.html".to_string());

                // Add resolution and source to content if available
                if let Some(res) = resolution {
                    result.metadata.file_size = Some(res);
                }
                if let Some(src) = source {
                    result.metadata.author = Some(src);
                }

                results.push(result);
                position += 1;
            }
        }

        Ok(EngineResults::with_results(results))
    }
}

/// Google News search engine
pub struct GoogleNews {
    base_url: String,
}

impl GoogleNews {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.google.com/search".to_string(),
        }
    }
}

impl Default for GoogleNews {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GoogleNews {
    fn name(&self) -> &str {
        "google_news"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://news.google.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["news"]
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("tbm".to_string(), "nws".to_string());
        query_params.insert("hl".to_string(), params.lang.clone());

        if let Some(ref time_range) = params.time_range {
            let tbs = match time_range {
                crate::query::TimeRange::Day => "qdr:d",
                crate::query::TimeRange::Week => "qdr:w",
                crate::query::TimeRange::Month => "qdr:m",
                crate::query::TimeRange::Year => "qdr:y",
            };
            query_params.insert("tbs".to_string(), tbs.to_string());
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let document = Html::parse_document(&response.text);
        let mut results = Vec::new();

        // News result parsing
        let result_selector = Selector::parse("div.SoaBEf, div.xuvV6b").unwrap();
        let title_selector = Selector::parse("div.mCBkyc, div.n0jPhd").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let snippet_selector = Selector::parse("div.GI74Re, div.Y3v8qd").unwrap();

        let mut position = 1u32;

        for element in document.select(&result_selector) {
            let title = element
                .select(&title_selector)
                .next()
                .map(|t| t.text().collect::<String>())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let url = element
                .select(&link_selector)
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|h| h.to_string())
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            let snippet = element
                .select(&snippet_selector)
                .next()
                .map(|s| s.text().collect::<String>());

            let mut result = Result::new(url, title, self.name().to_string());
            if let Some(content) = snippet {
                result = result.with_content(content);
            }
            result = result.with_position(position);
            result.result_type = crate::results::ResultType::News;
            position += 1;

            results.push(result);
        }

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_request() {
        let google = Google::new();
        let params = RequestParams::new("rust programming");
        let request = google.request(&params).unwrap();

        assert!(request.url.contains("google.com"));
        assert!(request.params.contains_key("q"));
    }

    /// A JS-gated interstitial must be reported as an error, not as an empty
    /// but successful result set. Regression test for the datacenter-IP case
    /// where Google answers 200 with a bootstrap page.
    #[test]
    fn js_gated_interstitial_is_an_error() {
        let google = Google::new();
        let html = r#"<!DOCTYPE html><html><head><title>Google Search</title></head>
            <body><noscript><meta content="0;url=/httpservice/retry/enablejs?sei=abc"
            http-equiv="refresh"></noscript>
            <div id="yvlrue">If you're having trouble accessing Google Search,
            please click here&emsg=SG_REL&sei=abc</div></body></html>"#;
        let response = EngineResponse {
            status: 200,
            headers: HashMap::new(),
            text: html.to_string(),
            url: "https://www.google.com/search?q=rust".to_string(),
        };

        assert!(response.is_js_gated());
        assert!(
            !response.is_captcha(),
            "gated page carries no CAPTCHA wording"
        );
        let error = google.response(response).unwrap_err().to_string();
        assert!(error.contains("Blocked"), "unexpected error: {error}");
    }

    /// A large body that matches no result container is a parse failure, so the
    /// engine gets reported as unresponsive instead of silently contributing
    /// zero results.
    #[test]
    fn oversized_unparsed_body_is_an_error() {
        let google = Google::new();
        let response = EngineResponse {
            status: 200,
            headers: HashMap::new(),
            text: format!("<html><body>{}</body></html>", "x".repeat(25_000)),
            url: "https://www.google.com/search?q=rust".to_string(),
        };

        let error = google.response(response).unwrap_err().to_string();
        assert!(error.contains("Failed to parse response"), "got: {error}");
    }

    /// A small, genuinely empty SERP stays a successful search with no results.
    #[test]
    fn small_empty_serp_is_not_an_error() {
        let google = Google::new();
        let response = EngineResponse {
            status: 200,
            headers: HashMap::new(),
            text: "<html><body><p>Your search did not match any documents.</p></body></html>"
                .to_string(),
            url: "https://www.google.com/search?q=zzzz".to_string(),
        };

        let results = google.response(response).unwrap();
        assert!(results.results.is_empty());
    }
}
