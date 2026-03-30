//! National Vulnerability Database (NVD) search engine implementation
//!
//! Provides access to CVE (Common Vulnerabilities and Exposures) data from the US
//! National Institute of Standards and Technology (NIST).

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// NVD search engine for security vulnerabilities
pub struct Nvd {
    base_url: String,
}

impl Nvd {
    pub fn new() -> Self {
        Self {
            base_url: "https://services.nvd.nist.gov/rest/json/cves/2.0".to_string(),
        }
    }

    /// Parse CVE JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let vulnerabilities = json_data
            .get("vulnerabilities")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in vulnerabilities {
            let cve_node = item.get("cve");
            let cve_data = match cve_node {
                Some(node) => node,
                None => continue,
            };

            // Get CVE ID
            let cve_id = cve_data
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or_default()
                .to_string();

            if cve_id.is_empty() {
                continue;
            }

            // Get description
            let descriptions = cve_data
                .get("descriptions")
                .and_then(|d| d.as_array())
                .cloned()
                .unwrap_or_default();

            let mut content_parts = Vec::new();
            for desc in descriptions {
                if desc.get("lang").and_then(|l| l.as_str()) == Some("en") {
                    if let Some(value) = desc.get("value").and_then(|v| v.as_str()) {
                        content_parts.push(value.to_string());
                        break;
                    }
                }
            }

            let content = if content_parts.is_empty() {
                None
            } else {
                Some(content_parts.join(" "))
            };

            // Get CVSS scores
            let metrics = cve_data.get("metrics");
            let severity = metrics.and_then(|m| {
                m.get("cvssMetricV31")
                    .or_else(|| m.get("cvssMetricV30"))
                    .or_else(|| m.get("cvssMetricV40"))
                    .or_else(|| m.get("cvssMetricV2"))
            });

            let severity_level = severity.and_then(|s| {
                s.as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|m| m.get("cvssData"))
                    .and_then(|d| d.get("baseSeverity"))
                    .and_then(|s| s.as_str())
            });

            let cvss_score = severity.and_then(|s| {
                s.as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|m| m.get("cvssData"))
                    .and_then(|d| d.get("baseScore"))
                    .and_then(|s| s.as_f64())
            });

            // Get published date
            let published_date = cve_data
                .get("published")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get modified date
            let modified_date = cve_data
                .get("lastModified")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get CWE (Common Weakness Enumeration)
            let cwe = cve_data
                .get("configurations")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|node| node.get("nodes"))
                .and_then(|nodes| nodes.as_array())
                .and_then(|arr| arr.first())
                .and_then(|node| node.get("cpeMatch"))
                .and_then(|cpe| cpe.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|match_item| {
                            match_item
                                .get("criteria")
                                .and_then(|c| c.as_str())
                                .map(|c| c.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                });

            // Build result URL
            let url = format!("https://nvd.nist.gov/vuln/detail/{}", cve_id);

            // Create result
            let mut result = Result::new(url, cve_id, self.name().to_string());
            result.result_type = ResultType::Security;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            if let Some(severity) = severity_level {
                result.metadata.severity = Some(severity.to_string());
            }

            if let Some(score) = cvss_score {
                result.metadata.cvss_score = Some(score);
            }

            if let Some(pub_date) = published_date {
                result.metadata.published_date = Some(pub_date);
            }

            if let Some(mod_date) = modified_date {
                result.metadata.modified_date = Some(mod_date);
            }

            if let Some(cwe_info) = cwe {
                if !cwe_info.is_empty() {
                    result.metadata.tags = Some(vec![cwe_info]);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Nvd {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Nvd {
    fn name(&self) -> &str {
        "nvd"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://nvd.nist.gov")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "security", "vulnerabilities"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search by keyword
        query_params.insert("keyword".to_string(), params.query.clone());

        // Pagination (NVD uses start index and length)
        let start = (params.pageno - 1) * 10;
        query_params.insert("startIndex".to_string(), start.to_string());
        query_params.insert("pageSize".to_string(), "10".to_string());

        // Query structure for CVE search
        let query_struct = format!(r#"{{"cveId":{{"$regex":"{}"}}}}"#, params.query);
        query_params.insert("cveId".to_string(), query_struct);

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
    fn test_nvd_request() {
        let nvd = Nvd::new();
        let params = RequestParams::new("rust");
        let request = nvd.request(&params).unwrap();

        assert!(request.url.contains("nvd.nist.gov"));
        assert!(request.params.contains_key("keyword"));
    }
}
