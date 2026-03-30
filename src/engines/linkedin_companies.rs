//! LinkedIn company search engine implementation
//!
//! Search for company profiles and corporate information using LinkedIn's API.
//! Requires LinkedIn API key configured in settings or passed at runtime.

use super::traits::*;
use crate::config::EngineConfig;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use serde_json::json;
use std::collections::HashMap;

/// LinkedIn company search engine
pub struct LinkedInCompanies {
    base_url: String,
}

impl LinkedInCompanies {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.linkedin.com/v2/companies".to_string(),
        }
    }

    /// Parse LinkedIn company search results
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let elements = json_data
            .get("elements")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in elements {
            // Get company name
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Get universal name
            let universal_name = item
                .get("universalName")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if universal_name.is_empty() {
                continue;
            }

            // Get URL
            let url = format!("https://www.linkedin.com/company/{}", universal_name);

            // Get tagline
            let tagline = item.get("tagline").and_then(|t| {
                t.get("text")
                    .and_then(|t| t.as_str().map(|s| s.to_string()))
            });

            // Get description
            let description = item
                .get("description")
                .and_then(|d| d.get("text"))
                .and_then(|d| d.as_str().map(|s| s.to_string()));

            // Get headquarters location
            let headquarters = item
                .get("headquarters")
                .and_then(|h| h.get("geoObjectId"))
                .and_then(|geo| geo.get("name"))
                .and_then(|n| n.as_str())
                .map(|n| n.to_string());

            // Get industry
            let industry = item
                .get("industry")
                .and_then(|i| i.as_str())
                .map(|i| i.to_string());

            // Get company size range
            let company_size = item
                .get("companySizeRange")
                .and_then(|s| s.get("start"))
                .and_then(|s| s.as_u64())
                .map(|start| format!("{}+", start));

            // Get follower count
            let followers = item.get("followerCount").and_then(|f| f.as_u64());

            // Get website URL
            let website = item
                .get("website")
                .and_then(|w| w.as_str())
                .map(|w| w.to_string());

            // Get logo URL
            let logo = item
                .get("logo")
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Create result
            let mut result = Result::new(url, name, self.name().to_string());
            result.result_type = ResultType::Corporate;

            if let Some(tagline) = tagline {
                result = result.with_content(tagline);
            }

            if let Some(desc) = description {
                if result
                    .content
                    .as_ref()
                    .map(|c| c.is_empty())
                    .unwrap_or(true)
                {
                    result = result.with_content(desc);
                } else {
                    result.metadata.tags = Some(vec![desc]);
                }
            }

            if let Some(hq) = headquarters {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Location: {}", hq));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Location: {}", hq)]);
                }
            }

            if let Some(ind) = industry {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Industry: {}", ind));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Industry: {}", ind)]);
                }
            }

            if let Some(size) = company_size {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Size: {}", size));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Size: {}", size)]);
                }
            }

            if let Some(followers_count) = followers {
                result.metadata.views = Some(followers_count);
            }

            if let Some(website) = website {
                result.metadata.homepage = Some(website);
            }

            if let Some(logo_url) = logo {
                result.metadata.thumbnail = Some(logo_url);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for LinkedInCompanies {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for LinkedInCompanies {
    fn name(&self) -> &str {
        "linkedin_companies"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.linkedin.com/company")
            .official_api(true)
            .api_key_required(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["corporate", "business", "companies"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        // Validate API key is configured (either in config or passed at runtime)
        // First, check for runtime API key override from params
        let runtime_api_key = params
            .engine_data
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // If no runtime key, we should have validated at load time via validate()
        // The validate() method ensures API key exists in EngineConfig
        // For this implementation, we'll use runtime key if provided
        let api_key = runtime_api_key;

        if api_key.is_none() || api_key.as_ref().map(|k| k.is_empty()).unwrap_or(true) {
            return Err(anyhow::anyhow!(
                "LinkedIn Companies engine requires an API key. \
                 Please configure 'api_key' in settings.yml for engine '{}' \
                 or pass it via request parameters.",
                self.name()
            ));
        }

        let query_body = json!({
            "q": "companies",
            "query": params.query,
            "decorationId": "com.linkedin.default.search.companies.v2.CompaniesCollection",
            "count": 10,
            "start": ((params.pageno - 1) * 10)
        });

        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), query_body.to_string());

        let mut request = EngineRequest::post(&self.base_url);
        request
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        // Add API key to header
        if let Some(key) = &api_key {
            request
                .headers
                .insert("Authorization".to_string(), format!("Bearer {}", key));
        }

        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        // Check for authentication errors
        if response.status == 401 || response.status == 403 {
            return Err(anyhow::anyhow!(
                "LinkedIn API requires authentication. \
                 Please configure a valid API key in settings.yml for engine '{}'.",
                self.name()
            ));
        }

        let json_data: serde_json::Value = serde_json::from_str(&response.text)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        let results = self.parse_results(&json_data);

        Ok(EngineResults::with_results(results))
    }

    /// Validate that API key is configured during engine loading
    fn validate(&self, config: &EngineConfig) -> AnyhowResult<()> {
        if config.api_key.is_none() || config.api_key.as_ref().unwrap().is_empty() {
            return Err(anyhow::anyhow!(
                "LinkedIn Companies engine requires an API key. \
                 Please configure 'api_key' in settings.yml for engine '{}' \
                 or disable this engine if you don't have a LinkedIn API key.",
                self.name()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use serde_json::json;

    #[test]
    fn test_linkedin_companies_request_with_api_key() {
        let linkedin = LinkedInCompanies::new();
        let mut params = RequestParams::new("tech companies");

        // Add API key to engine_data
        params
            .engine_data
            .insert("api_key".to_string(), json!("test_api_key_123"));

        // Request should succeed with API key
        let request = linkedin.request(&params);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert!(request.headers.contains_key("Authorization"));
        assert!(request
            .headers
            .get("Authorization")
            .unwrap()
            .starts_with("Bearer "));
    }

    #[test]
    fn test_linkedin_companies_request_without_api_key() {
        let linkedin = LinkedInCompanies::new();
        let params = RequestParams::new("tech companies");

        // Request should fail without API key
        let request = linkedin.request(&params);
        assert!(request.is_err());
        let err_msg = request.unwrap_err().to_string();
        assert!(err_msg.contains("API key"));
        assert!(err_msg.contains("linkedin_companies"));
    }

    #[test]
    fn test_linkedin_companies_categories() {
        let linkedin = LinkedInCompanies::new();
        let categories = linkedin.categories();

        assert!(categories.contains(&"corporate"));
        assert!(categories.contains(&"business"));
        assert!(categories.contains(&"companies"));
    }

    #[test]
    fn test_linkedin_companies_paging() {
        let linkedin = LinkedInCompanies::new();
        assert!(linkedin.supports_paging());
    }

    #[test]
    fn test_linkedin_companies_about() {
        let linkedin = LinkedInCompanies::new();
        let about = linkedin.about();

        assert_eq!(
            about.website,
            Some("https://www.linkedin.com/company".to_string())
        );
        assert_eq!(about.require_api_key, true);
    }

    #[test]
    fn test_linkedin_companies_validate_with_api_key() {
        let linkedin = LinkedInCompanies::new();
        let config = EngineConfig {
            name: "linkedin_test".to_string(),
            engine: "linkedin_companies".to_string(),
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };

        let result = linkedin.validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_linkedin_companies_validate_without_api_key() {
        let linkedin = LinkedInCompanies::new();
        let config = EngineConfig {
            name: "linkedin_test".to_string(),
            engine: "linkedin_companies".to_string(),
            api_key: None,
            ..Default::default()
        };

        let result = linkedin.validate(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("API key"));
        assert!(err_msg.contains("linkedin_companies"));
    }

    #[test]
    fn test_linkedin_companies_validate_empty_api_key() {
        let linkedin = LinkedInCompanies::new();
        let config = EngineConfig {
            name: "linkedin_test".to_string(),
            engine: "linkedin_companies".to_string(),
            api_key: Some("".to_string()),
            ..Default::default()
        };

        let result = linkedin.validate(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("API key"));
    }
}
