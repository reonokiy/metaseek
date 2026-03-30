//! SEC EDGAR search engine implementation
//!
//! Search for public company filings and corporate data using SEC EDGAR's free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// SEC EDGAR public company filings search engine
pub struct SecEdgar {
    base_url: String,
}

impl SecEdgar {
    pub fn new() -> Self {
        Self {
            base_url: "https://data.sec.gov/api/v1/search".to_string(),
        }
    }

    /// Parse SEC EDGAR JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let filings = json_data
            .get("filings")
            .and_then(|f| f.get("fillings"))
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in filings {
            // Get company name
            let company_name = item
                .get("companyName")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if company_name.is_empty() {
                continue;
            }

            // Get filing type
            let filing_type = item
                .get("form")
                .and_then(|f| f.as_str())
                .unwrap_or("Unknown")
                .to_string();

            // Get filing date
            let filing_date = item
                .get("filingDate")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get document count
            let document_count = item
                .get("documents")
                .and_then(|d| d.as_array())
                .map(|arr| arr.len() as u64);

            // Get SEC file number
            let sec_file_number = item
                .get("fileNumber")
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());

            // Construct URL
            let url = item
                .get("accessionNumber")
                .and_then(|a| a.as_str())
                .map(|a| format!("https://www.sec.gov/Archives/edgar/{}/{}.txt", a, a))
                .unwrap_or_default();

            if url.is_empty() {
                continue;
            }

            // Create result
            let mut result = Result::new(url, company_name.clone(), self.name().to_string());
            result.result_type = ResultType::Paper;

            // Build content description
            let mut content_parts = Vec::new();
            if !filing_type.is_empty() {
                content_parts.push(format!("Form: {}", filing_type));
            }
            if let Some(doc_count) = document_count {
                content_parts.push(format!("Documents: {}", doc_count));
            }
            if let Some(sec_num) = &sec_file_number {
                content_parts.push(format!("SEC File #: {}", sec_num));
            }

            if let Some(content) = content_parts.join(" | ").into() {
                result = result.with_content(content);
            }

            if let Some(file_date) = filing_date {
                result.metadata.published_date = Some(file_date);
            }

            if let Some(file_num) = &sec_file_number {
                result.metadata.template = Some(file_num.clone());
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for SecEdgar {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SecEdgar {
    fn name(&self) -> &str {
        "sec_edgar"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.sec.gov/edgar.htm")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["corporate", "business", "financial", "sec", "filings"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search by company name or ticker
        query_params.insert("companyName".to_string(), params.query.clone());

        // Pagination
        let _start = (params.pageno - 1) * 10;
        query_params.insert("fromDate".to_string(), "2020-01-01".to_string());
        query_params.insert("toDate".to_string(), "2024-12-31".to_string());

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
    fn test_sec_edgar_request() {
        let sec = SecEdgar::new();
        let params = RequestParams::new("Apple");
        let request = sec.request(&params).unwrap();

        assert!(request.url.contains("sec.gov"));
        assert!(request.params.contains_key("companyName"));
    }
}
