//! Crunchbase search engine implementation
//!
//! Search for startup and company data using Crunchbase's free tier API.
//! Note: This uses the public Crunchbase search which doesn't require an API key.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
/// Crunchbase company and startup search engine
pub struct Crunchbase {}

impl Crunchbase {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse Crunchbase HTML search results
    fn parse_results(&self, html: &str) -> Vec<Result> {
        let document = scraper::Html::parse_document(html);
        let mut results = Vec::new();

        // Selectors for Crunchbase search results
        let result_selector = scraper::Selector::parse("div.sb-entity-preview").unwrap();
        let title_selector = scraper::Selector::parse("h1.sb-title").unwrap();
        let link_selector = scraper::Selector::parse("a.sb-entity-preview-link").unwrap();
        let description_selector = scraper::Selector::parse("p.sb-description").unwrap();

        let mut position = 1u32;

        for element in document.select(&result_selector) {
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
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|u| format!("https://www.crunchbase.com{}", u))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Get description
            let description = element
                .select(&description_selector)
                .next()
                .map(|d| d.text().collect::<String>().trim().to_string())
                .filter(|d: &String| !d.is_empty());

            // Get funding info
            let funding = element
                .select(&scraper::Selector::parse(".funding-amount").unwrap())
                .next()
                .and_then(|f| Some(f.text().collect::<String>().trim().to_string()))
                .filter(|f: &String| !f.is_empty());

            // Get location
            let location = element
                .select(&scraper::Selector::parse(".location").unwrap())
                .next()
                .and_then(|l| Some(l.text().collect::<String>().trim().to_string()))
                .filter(|l: &String| !l.is_empty());

            // Get company type
            let company_type = element
                .select(&scraper::Selector::parse(".company-type").unwrap())
                .next()
                .and_then(|t| Some(t.text().collect::<String>().trim().to_string()))
                .filter(|t: &String| !t.is_empty());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Default;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            if let Some(fund) = funding {
                if let Some(existing) = result.metadata.tags.as_mut() {
                    existing.insert(0, format!("Funding: {}", fund));
                } else {
                    result.metadata.tags = Some(vec![format!("Funding: {}", fund)]);
                }
            }

            if let Some(loc) = location {
                if let Some(existing) = result.metadata.tags.as_mut() {
                    existing.push(format!("Location: {}", loc));
                } else {
                    result.metadata.tags = Some(vec![format!("Location: {}", loc)]);
                }
            }

            if let Some(corp_type) = company_type {
                if let Some(existing) = result.metadata.tags.as_mut() {
                    existing.push(format!("Type: {}", corp_type));
                } else {
                    result.metadata.tags = Some(vec![format!("Type: {}", corp_type)]);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Crunchbase {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Crunchbase {
    fn name(&self) -> &str {
        "crunchbase"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.crunchbase.com/")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec![
            "corporate",
            "business",
            "startups",
            "investments",
            "companies",
        ]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        use url::Url;

        let search_url =
            Url::parse_with_params("https://www.crunchbase.com/search", &[("q", &params.query)])
                .map_err(|e| anyhow::anyhow!("Failed to parse URL: {}", e))?;

        let mut request = EngineRequest::get(search_url.to_string());

        // Add pagination
        if params.pageno > 1 {
            request = request.param("page", params.pageno.to_string());
        }

        // Add headers to appear as a real browser
        request = request
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Connection", "keep-alive");

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
    fn test_crunchbase_request() {
        let cb = Crunchbase::new();
        let params = RequestParams::new("artificial intelligence startups");
        let request = cb.request(&params).unwrap();

        assert!(request.url.contains("crunchbase.com"));
        assert!(request.url.contains("q=artificial+intelligence+startups"));
    }
}
