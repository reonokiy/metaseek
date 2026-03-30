//! Wall Street Journal corporate and financial news engine
//!
//! Provides access to WSJ articles about companies and financial markets.
//!
//! # Status: DISABLED
//!
//! This engine has been disabled due to:
//! - No Python implementation exists in `python-engines/wsj.py`
//! - The engine was invented without proper validation
//! - Requires API key configuration for production use
//!
//! # Implementation Requirements
//!
//! Before re-enabling this engine, the following must be completed:
//!
//! 1. Create Python implementation in `python-engines/wsj.py`
//! 2. Validate the engine against WSJ's actual API/website
//! 3. Determine if API key is required (WSJ may require authentication)
//! 4. If API key is required, integrate with the common API key trait system
//! 5. Update engine metadata to reflect actual requirements
//!
//! # Current Issues
//!
//! - HTTP 401 errors when accessing WSJ without proper authentication
//! - No verified Python implementation exists
//! - Engine was added without proper validation process

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Wall Street Journal search engine
///
/// **DISABLED**: This engine is not functional until implementation is completed.
/// See module documentation for requirements.
pub struct WallStreetJournal {
    base_url: String,
}

impl WallStreetJournal {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.wsj.com".to_string(),
        }
    }

    /// Parse WSJ results from HTML
    ///
    /// # TODO
    ///
    /// Implement this method once the Python implementation is validated.
    /// Currently, WSJ requires proper authentication and may have
    /// rate limiting or paywall restrictions.
    fn parse_results(&self, _html: &str) -> Vec<Result> {
        todo!("Implement WSJ result parsing - requires validation of WSJ's HTML structure");
    }
}

impl Default for WallStreetJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for WallStreetJournal {
    fn name(&self) -> &str {
        "wsj"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.wsj.com")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["news", "financial", "corporate"]
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

        // WSJ uses different pagination
        if params.pageno > 1 {
            query_params.insert("offset".to_string(), ((params.pageno - 1) * 10).to_string());
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
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Connection", "keep-alive");

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        // Check for HTTP errors
        if !response.is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error: {} - WSJ requires authentication. Engine disabled until Python implementation is validated.",
                response.status
            ));
        }

        // Parse results
        let results = self.parse_results(&response.text);
        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Engine is disabled - requires Python implementation validation"]
    fn test_wsj_request() {
        let wsj = WallStreetJournal::new();
        let params = RequestParams::new("market analysis");
        let request = wsj.request(&params).unwrap();

        assert!(request.url.contains("wsj.com"));
        assert!(request.params.contains_key("q"));
    }
}
