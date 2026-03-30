//! Gitea instance search engine implementation
//!
//! Search for repositories on Gitea/Forgejo instances.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// Gitea package search engine
pub struct Gitea {
    base_url: String,
}

impl Gitea {
    pub fn new() -> Self {
        Self {
            base_url: "https://codeberg.org".to_string(), // Default to Codeberg (public Gitea instance)
        }
    }

    /// Set custom base URL for Gitea instance
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Parse Gitea JSON repository results
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let repositories = json_data
            .get("data")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in repositories {
            // Get full name
            let full_name = item
                .get("full_name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();

            if full_name.is_empty() {
                continue;
            }

            // Get name (already captured above, skip duplicate)
            let _name = full_name.clone();

            // Get name (already captured in full_name above)
            let _name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default();

            // Get description
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get HTML URL
            let html_url = item
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string();

            if html_url.is_empty() {
                continue;
            }

            // Get clone URL
            let clone_url = item
                .get("clone_url")
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get SSH URL
            let ssh_url = item
                .get("ssh_url")
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get default branch
            let default_branch = item
                .get("default_branch")
                .and_then(|b| b.as_str())
                .map(|b| b.to_string());

            // Get language
            let language = item
                .get("language")
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get stars count
            let stars = item.get("stars_count").and_then(|s| s.as_u64());

            // Get forks count
            let forks = item.get("forks_count").and_then(|f| f.as_u64());

            // Get issues count
            let _issues = item.get("open_issues_count").and_then(|i| i.as_u64());

            // Get size
            let _size = item.get("size").and_then(|s| s.as_f64());

            // Get created at
            let created_at = item
                .get("created_at")
                .and_then(|c| c.as_str())
                .map(|c| c.to_string());

            // Get updated at
            let updated_at = item
                .get("updated_at")
                .and_then(|d| d.as_str())
                .map(|d| d.to_string());

            // Get pushed at (already captured in updated_at above)
            let _pushed_at = item
                .get("pushed_at")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            // Get owner
            let owner = item
                .get("owner")
                .and_then(|o| o.get("username").or_else(|| o.get("login")))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get topics
            let topics = item.get("topics").and_then(|t| t.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

            // Get license
            let license = item
                .get("license")
                .and_then(|l| l.as_str())
                .map(|l| l.to_string());

            // Get permissions
            let _permissions = item.get("permissions");

            // Create result
            let mut result = Result::new(html_url, full_name, self.name().to_string());
            result.result_type = ResultType::Code;

            if let Some(desc) = description {
                result = result.with_content(desc);
            }

            result.metadata.version = default_branch;
            result.metadata.template = Some("packages.html".to_string());

            if let Some(lang) = language {
                result.metadata.tags = Some(vec![lang]);
            }

            if let Some(stars) = stars {
                result.metadata.stars = Some(stars);
            }

            if let Some(forks) = forks {
                result.metadata.forks = Some(forks);
            }

            if let Some(created) = created_at {
                result.metadata.published_date = Some(created);
            }

            if let Some(updated) = updated_at {
                result.metadata.last_update = Some(updated);
            }

            if let Some(owner) = owner {
                result.metadata.author = Some(owner);
            }

            if let Some(topics_vec) = topics {
                if !topics_vec.is_empty() {
                    if let Some(existing_tags) = &mut result.metadata.tags {
                        existing_tags.extend(topics_vec);
                    } else {
                        result.metadata.tags = Some(topics_vec);
                    }
                }
            }

            if let Some(lic) = license {
                result.metadata.license = Some(lic);
            }

            if let Some(clone) = clone_url {
                result.metadata.source_code = Some(clone);
            }

            if let Some(ssh) = ssh_url {
                result.metadata.ssh_url = Some(ssh);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Gitea {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Gitea {
    fn name(&self) -> &str {
        "gitea"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://gitea.io")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["it", "repos", "code"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();

        // Search query
        query_params.insert("q".to_string(), params.query.clone());

        // Pagination
        let page = params.pageno;
        let limit = 10;
        query_params.insert("page".to_string(), page.to_string());
        query_params.insert("limit".to_string(), limit.to_string());

        // Sort options
        query_params.insert("sort".to_string(), "updated".to_string());
        query_params.insert("order".to_string(), "desc".to_string());

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
    fn test_gitea_request() {
        let gitea = Gitea::new();
        let params = RequestParams::new("rust");
        let request = gitea.request(&params).unwrap();

        assert!(request.url.contains("codeberg.org"));
        assert!(request.params.contains_key("q"));
    }
}
