//! Protein Data Bank in Europe (PDBe) search engine implementation
//!
//! Search for 3D structural biology data including proteins and nucleic acids.
//! Reference: python-engines/pdbe.py

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// PDBe (Protein Data Bank in Europe) search engine
pub struct PDBe {
    base_url: String,
}

impl PDBe {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.ebi.ac.uk/pdbe/search/pdb/select".to_string(),
        }
    }

    /// Parse PDBe JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        // Get response.docs array
        let docs = json_data
            .get("response")
            .and_then(|r| r.get("docs"))
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let position = 1u32;

        for item in docs {
            // Skip unpublished entries
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or_default();

            let unpublished_codes = [
                "HPUB", "HOLD", "PROC", "WAIT", "AUTH", "AUCO", "REPL", "POLC", "REFI", "TRSF",
                "WDRN",
            ];
            if unpublished_codes.contains(&status) {
                continue;
            }

            // Get title
            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string();

            if title.is_empty() {
                continue;
            }

            // Get PDB ID
            let pdb_id = item
                .get("pdb_id")
                .and_then(|p| p.as_str())
                .unwrap_or_default()
                .to_string();

            if pdb_id.is_empty() {
                continue;
            }

            // Build URL
            let url = format!(
                "https://www.ebi.ac.uk/pdbe/entry/pdb/{}",
                pdb_id
            );

            // Extract authors (first author from list)
            let authors = item
                .get("entry_author_list")
                .and_then(|a| a.as_array())
                .and_then(|arr| arr.first())
                .and_then(|f| f.as_str())
                .map(|s| s.to_string());

            // Extract citation info
            let citation_title = item
                .get("citation_title")
                .and_then(|c| c.as_str())
                .map(|c| c.to_string());

            let journal = item
                .get("journal")
                .and_then(|j| j.as_str())
                .map(|j| j.to_string());

            let volume = item
                .get("journal_volume")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            let page = item
                .get("journal_page")
                .and_then(|p| p.as_str())
                .map(|p| p.to_string());

            let year = item
                .get("citation_year")
                .or_else(|| item.get("release_year"))
                .and_then(|y| y.as_str())
                .map(|y| y.to_string());

            // Build content/description
            let content = if let Some(journal) = &journal {
                if let Some(year) = &year {
                    if let Some(volume) = &volume {
                        format!(
                            "{} - {} {} ({}), {}",
                            citation_title.as_deref().unwrap_or(""),
                            authors.as_deref().unwrap_or(""),
                            journal,
                            volume,
                            page.as_deref().unwrap_or("")
                        )
                        .trim()
                        .to_string()
                    } else {
                        format!(
                            "{} - {} {}, {}",
                            citation_title.as_deref().unwrap_or(""),
                            authors.as_deref().unwrap_or(""),
                            journal,
                            year
                        )
                        .trim()
                        .to_string()
                    }
                } else {
                    citation_title.unwrap_or_default()
                }
            } else {
                citation_title.unwrap_or_default()
            };

            // Extract thumbnail URL
            let thumbnail = format!(
                "https://www.ebi.ac.uk/pdbe/static/entry/{}_deposited_chain_front_image-200x200.png",
                pdb_id
            );

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Paper;
            result = result.with_position(position);

            if !content.is_empty() {
                result = result.with_content(content);
            }

            if let Some(authors) = authors {
                result.metadata.author = Some(authors);
            }

            result.metadata.img_src = Some(thumbnail);

            // Add journal info to tags
            if let Some(j) = journal {
                if !j.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Journal: {}", j));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Journal: {}", j)]);
                    }
                }
            }

            if let Some(vol) = volume {
                if !vol.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Volume: {}", vol));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Volume: {}", vol)]);
                    }
                }
            }

            if let Some(pag) = page {
                if !pag.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Page: {}", pag));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Page: {}", pag)]);
                    }
                }
            }

            if let Some(y) = year {
                if !y.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("Year: {}", y));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("Year: {}", y)]);
                    }
                }
            }

            results.push(result);
        }

        results
    }
}

impl Default for PDBe {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for PDBe {
    fn name(&self) -> &str {
        "pdbe"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://www.ebi.ac.uk/pdbe")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "biology", "chemistry"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), params.query.clone());
        query_params.insert("wt".to_string(), "json".to_string());

        let mut request = EngineRequest::post(&self.base_url);
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
    fn test_pdbe_request() {
        let pdbe = PDBe::new();
        let params = RequestParams::new("hemoglobin structure");
        let request = pdbe.request(&params).unwrap();

        assert!(request.url.contains("ebi.ac.uk"));
        assert_eq!(request.method, HttpMethod::Post);
        assert!(request.params.contains_key("q"));
        assert_eq!(request.params.get("wt"), Some(&"json".to_string()));
    }

    #[test]
    fn test_pdbe_categories() {
        let pdbe = PDBe::new();
        let categories = pdbe.categories();

        assert!(categories.contains(&"science"));
        assert!(categories.contains(&"biology"));
        assert!(categories.contains(&"chemistry"));
    }

    #[test]
    fn test_pdbe_paging() {
        let pdbe = PDBe::new();
        assert!(pdbe.supports_paging());
    }
}
