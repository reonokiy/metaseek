//! Google Scholar search engine implementation
//!
//! Search for academic papers and scholarly literature using Google Scholar.
//! Reference: python-engines/google_scholar.py

use super::traits::*;
#[allow(unused_imports)]
use crate::query::TimeRange;
use crate::results::Result;
use anyhow::Result as AnyhowResult;
use chrono::Datelike;
use std::collections::HashMap;

/// Google Scholar search engine for academic papers
pub struct GoogleScholar {
    base_url: String,
}

impl GoogleScholar {
    pub fn new() -> Self {
        Self {
            base_url: "https://scholar.google.com".to_string(),
        }
    }

    /// Parse the gs_a field to extract authors, journal, publisher, and year
    #[allow(dead_code)]
    fn parse_gs_a(&self, text: &str) -> (Vec<String>, String, String, Option<String>) {
        if text.is_empty() {
            return (Vec::new(), String::new(), String::new(), None);
        }

        let parts: Vec<&str> = text.split(" - ").collect();
        let authors: Vec<String> = parts[0].split(", ").map(|s| s.to_string()).collect();
        let publisher = parts.last().unwrap_or(&"").to_string();

        if parts.len() != 3 {
            return (authors, String::new(), publisher, None);
        }

        // Format: "{authors} - {journal}, {year} - {publisher}" or "{authors} - {year} - {publisher}"
        let journal_year: Vec<&str> = parts[1].split(", ").collect();
        let journal: String = if journal_year.len() > 1 {
            let j = journal_year[..journal_year.len() - 1].join(", ");
            if j == "…" {
                String::new()
            } else {
                j
            }
        } else {
            String::new()
        };

        let year = journal_year.last().map(|s| s.trim().to_string());

        (authors, journal, publisher, year)
    }

    /// Parse HTML response from Google Scholar
    fn parse_results(&self, _html_content: &str) -> Vec<Result> {
        // Simple implementation - just return empty results for now
        // Full implementation would require proper HTML parsing
        Vec::new()
    }
}

impl Default for GoogleScholar {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for GoogleScholar {
    fn name(&self) -> &str {
        "google_scholar"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://scholar.google.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "scientific publications"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Main search query
        query_params.insert("q".to_string(), params.query.clone());

        // Pagination (start parameter)
        let start = (params.pageno - 1) * 10;
        query_params.insert("start".to_string(), start.to_string());

        // Search scope - scholarly articles (include patents)
        query_params.insert("as_sdt".to_string(), "2007".to_string());

        // Visibility - include citations
        query_params.insert("as_vis".to_string(), "0".to_string());

        // Time range support
        if let Some(time_range) = params.time_range {
            // Google Scholar maps all time ranges to year-based filtering
            let current_year = chrono::Utc::now().year();
            query_params.insert("as_ylo".to_string(), (current_year - 1).to_string());

            // Note: Google Scholar only supports limiting to past year, not specific ranges
            // So we use the same logic for all time ranges
            let _ = time_range; // Suppress unused warning
        }

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        // Check for CAPTCHA or access denied
        if response.text.contains("captcha")
            || response.text.contains("unusual traffic")
            || response.text.contains("gs_captcha_f")
        {
            return Err(anyhow::anyhow!(
                "Google Scholar detected unusual traffic or CAPTCHA"
            ));
        }

        let results = self.parse_results(&response.text);

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_scholar_request() {
        let gs = GoogleScholar::new();
        let params = RequestParams::new("machine learning");
        let request = gs.request(&params).unwrap();

        assert!(request.url.contains("scholar.google.com"));
        assert!(request.params.contains_key("q"));
        assert!(request.params.contains_key("start"));
        assert_eq!(request.params.get("as_sdt"), Some(&"2007".to_string()));
    }

    #[test]
    fn test_google_scholar_time_range() {
        let gs = GoogleScholar::new();
        let mut params = RequestParams::new("quantum computing");
        params.time_range = Some(TimeRange::Year);
        let request = gs.request(&params).unwrap();

        assert!(request.params.contains_key("as_ylo"));
    }

    #[test]
    fn test_google_scholar_categories() {
        let gs = GoogleScholar::new();
        let categories = gs.categories();

        assert!(categories.contains(&"science"));
        assert!(categories.contains(&"scientific publications"));
    }

    #[test]
    fn test_google_scholar_paging() {
        let gs = GoogleScholar::new();

        assert!(gs.supports_paging());
        assert!(gs.supports_time_range());
    }

    #[test]
    fn test_parse_gs_a() {
        let gs = GoogleScholar::new();

        // Test format: authors - journal, year - publisher
        let (_authors, journal, publisher, year) =
            gs.parse_gs_a("Smith, John, Doe, Jane - Nature, 2023 - Publisher X");
        assert_eq!(journal, "Nature");
        assert_eq!(year, Some("2023".to_string()));
        assert_eq!(publisher, "Publisher X");

        // Test format: authors - year - publisher
        let (_authors, journal, publisher, year) =
            gs.parse_gs_a("Smith, John - 2023 - Publisher X");
        assert_eq!(journal, "");
        assert_eq!(year, Some("2023".to_string()));
        assert_eq!(publisher, "Publisher X");

        // Test empty input
        let (_authors, journal, publisher, year) = gs.parse_gs_a("");
        assert!(journal.is_empty());
        assert!(publisher.is_empty());
        assert!(year.is_none());
    }
}
