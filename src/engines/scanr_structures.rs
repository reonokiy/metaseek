//! SCANR Structures search engine implementation
//!
//! Search for French academic structures and research organizations.
//! Reference: python-engines/scanr_structures.py

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// SCANR Structures academic search engine
pub struct ScanrStructures {
    base_url: String,
}

impl ScanrStructures {
    pub fn new() -> Self {
        Self {
            base_url: "https://scanr.search.grenoble.cnrs.fr/api/structures/search".to_string(),
        }
    }

    /// Parse SCANR Structures JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let data = json_data
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in data {
            // Get identifier (uid)
            let uid = item.get("uid").and_then(|u| u.as_str()).unwrap_or_default();

            if uid.is_empty() {
                continue;
            }

            // Get title/name
            let title = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if title.is_empty() {
                continue;
            }

            // Build URL
            let url = format!(
                "https://scanr.search.grenoble.cnrs.fr/explore/entity/scanr_structures_0/uid/{}",
                uid
            );

            // Extract description
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get type
            let struct_type = item
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t.to_string());

            // Get parent organizations
            let parents = item.get("parents").and_then(|p| p.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            });

            // Extract location
            let location = item
                .get("locations")
                .and_then(|l| l.as_array())
                .and_then(|arr| arr.first())
                .and_then(|l| l.get("city"))
                .and_then(|c| c.as_str())
                .map(|c| c.to_string());

            // Extract website
            let website = item
                .get("website")
                .and_then(|w| w.as_str())
                .map(|w| w.to_string());

            // Get statistics
            let research_units = item.get("research_units_count").and_then(|r| r.as_u64());

            let publications_count = item.get("publications_count").and_then(|p| p.as_u64());

            let researchers_count = item.get("researchers_count").and_then(|r| r.as_u64());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Corporate;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            // Add metadata
            if let Some(stype) = struct_type {
                if !stype.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Type: {}", stype));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Type: {}", stype)]);
                    }
                }
            }

            if let Some(parents) = parents {
                if !parents.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Parent: {}", parents));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Parent: {}", parents)]);
                    }
                }
            }

            if let Some(loc) = location {
                if !loc.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Location: {}", loc));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Location: {}", loc)]);
                    }
                }
            }

            if let Some(website) = website {
                result.metadata.homepage = Some(website);
            }

            if let Some(stats) = research_units {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Research Units: {}", stats));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Research Units: {}", stats)]);
                }
            }

            if let Some(pub_count) = publications_count {
                result.metadata.views = Some(pub_count);
            }

            if let Some(researchers) = researchers_count {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Researchers: {}", researchers));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Researchers: {}", researchers)]);
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for ScanrStructures {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for ScanrStructures {
    fn name(&self) -> &str {
        "scanr_structures"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://scanr.search.grenoble.cnrs.fr")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "academic"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let query = if params.pageno == 1 {
            params.query.clone()
        } else {
            // Add offset for pagination
            format!("{} offset:{}", params.query, (params.pageno - 1) * 10)
        };

        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), query);
        query_params.insert("size".to_string(), "10".to_string());

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
    fn test_scanr_structures_request() {
        let ssc = ScanrStructures::new();
        let params = RequestParams::new("CNRS research");
        let request = ssc.request(&params).unwrap();

        assert!(request.url.contains("scanr.search.grenoble.cnrs.fr"));
        assert!(request.params.contains_key("q"));
        assert_eq!(request.params.get("size"), Some(&"10".to_string()));
    }

    #[test]
    fn test_scanr_structures_categories() {
        let ssc = ScanrStructures::new();
        let categories = ssc.categories();

        assert!(categories.contains(&"science"));
        assert!(categories.contains(&"academic"));
    }

    #[test]
    fn test_scanr_structures_paging() {
        let ssc = ScanrStructures::new();
        assert!(ssc.supports_paging());
    }
}
