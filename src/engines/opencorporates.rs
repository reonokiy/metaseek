//! OpenCorporates search engine implementation
//!
//! Search for corporate entities using OpenCorporates' free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// OpenCorporates global corporate registry search engine
pub struct OpenCorporates {
    base_url: String,
}

impl OpenCorporates {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.opencorporates.com/api/v0.4/companies".to_string(),
        }
    }

    /// Parse OpenCorporates JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let companies = json_data
            .get("results")
            .and_then(|r| r.get("companies"))
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in companies {
            // Get company number
            let company_number = item
                .get("company_number")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if company_number.is_empty() {
                continue;
            }

            // Get company name
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Get URL
            let url = format!(
                "https://opencorporates.com/companies/{}",
                company_number
            );

            // Get jurisdiction code
            let jurisdiction = item
                .get("jurisdiction_code")
                .and_then(|j| j.as_str())
                .map(|j| j.to_string());

            // Get incorporation date
            let incorporation_date = item
                .get("incorporation_date")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get status
            let status = item
                .get("company_status")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            // Get registered address
            let address = item
                .get("registered_address")
                .and_then(|a| a.as_str())
                .map(|a| a.to_string());

            // Get links
            let links = item
                .get("links")
                .and_then(|l| l.as_object())
                .and_then(|l| l.get("self"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, name.clone(), self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(status) = status {
                if !status.is_empty() {
                    result.metadata.template = Some(status.to_lowercase());
                }
            }

            if let Some(incorp) = incorporation_date {
                result.metadata.published_date = Some(incorp);
            }

            if let Some(addr) = address {
                if !addr.is_empty() {
                    result.metadata.tags = Some(vec![addr]);
                }
            }

            if let Some(jurisdiction) = jurisdiction {
                if !jurisdiction.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.insert(0, format!("Jurisdiction: {}", jurisdiction));
                    } else {
                        result.metadata.tags = Some(vec![format!("Jurisdiction: {}", jurisdiction)]);
                    }
                }
            }

            if let Some(link) = links {
                if !link.is_empty() {
                    result.metadata.homepage = Some(link);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for OpenCorporates {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for OpenCorporates {
    fn name(&self) -> &str {
        "opencorporates"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://opencorporates.com/")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["corporate", "business", "companies", "entities"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("per_page".to_string(), "10".to_string());
        query_params.insert("page".to_string(), params.pageno.to_string());

        // Build search query
        let query_body = format!(
            r#"{{"query":"{}","format":"json"}}"#,
            params.query
        );
        query_params.insert("q".to_string(), query_body);

        let mut request = EngineRequest::get(&self.base_url);
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        let json_data: serde_json::Value = serde_json::from_str(&response.text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let results = self.parse_results(&json_data);

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencorporates_request() {
        let oc = OpenCorporates::new();
        let params = RequestParams::new("Google");
        let request = oc.request(&params).unwrap();

        assert!(request.url.contains("opencorporates.com"));
        assert!(request.params.contains_key("q"));
    }
}
