//! OpenAlex research engine implementation
//!
//! Search for academic papers and research using OpenAlex's free API.

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// OpenAlex academic paper search engine
pub struct OpenAlex {
    base_url: String,
}

impl OpenAlex {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.openalex.org/works".to_string(),
        }
    }

    /// Parse OpenAlex JSON response
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let items = json_data
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        for item in items {
            // Get title
            let title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string();

            if title.is_empty() {
                continue;
            }

            // Get URL
            let url = item
                .get("url")
                .and_then(|u| u.as_str())
                .or_else(|| item.get("open_url").and_then(|u| u.as_str()))
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                continue;
            }

            // Get abstract (reconstructed from inverted index)
            let abstract_text = item
                .get("abstract_inverted_index")
                .and_then(|a| a.as_object())
                .map(|idx| {
                    let mut words: Vec<String> = vec![String::new(); idx.len()];
                    for (word, positions) in idx {
                        if let Some(pos_list) = positions.as_array() {
                            for pos in pos_list {
                                if let Some(&pos) = pos.as_u64().map(|p| p as usize).as_ref() {
                                    if pos < words.len() {
                                        words[pos] = word.clone();
                                    }
                                }
                            }
                        }
                    }
                    words.join(" ")
                });

            // Get authors
            let authors = item
                .get("authorships")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|authorship| {
                            authorship
                                .get("author")
                                .and_then(|a| a.get("display_name"))
                                .and_then(|n| n.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .map(|s| s.to_string());

            // Get journal/venue
            let journal = item
                .get("host_venue")
                .and_then(|v| v.get("display_name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            // Get publisher
            let publisher = item
                .get("host_venue")
                .and_then(|v| v.get("publisher"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());

            // Get publication date
            let published_date = item
                .get("publication_date")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

            // Get DOI
            let doi = item
                .get("doi")
                .and_then(|d| d.as_str())
                .map(|d| d.trim_start_matches("https://doi.org/").to_string());

            // Get tags/concepts
            let tags = item.get("concepts").and_then(|c| c.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        c.get("display_name")
                            .and_then(|n| n.as_str().map(|s| s.to_string()))
                    })
                    .collect::<Vec<_>>()
            });

            // Get citation count
            let citations = item.get("cited_by_count").and_then(|c| c.as_u64());

            // Get PDF URL
            let pdf_url = item
                .get("latest_oa_versions")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.get("pdf_url"))
                .and_then(|u| u.as_str())
                .map(|u| u.to_string());

            // Get HTML URL
            let html_url = item
                .get("oai")
                .and_then(|o| o.as_str())
                .or_else(|| item.get("landing_page_url").and_then(|u| u.as_str()))
                .map(|u| u.to_string());

            // Get type (unused, skipped)
            let _paper_type = item
                .get("type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            // Create result
            let mut result = Result::new(url, title, self.name().to_string());
            result.result_type = ResultType::Paper;

            if let Some(abstract_text) = abstract_text {
                result = result.with_content(abstract_text);
            }

            if let Some(authors) = authors {
                result.metadata.author = Some(authors);
            }

            if let Some(pub_date) = published_date {
                result.metadata.published_date = Some(pub_date);
            }

            if let Some(citations) = citations {
                result.metadata.views = Some(citations);
            }

            if let Some(doi) = doi {
                result.metadata.tags = Some(vec![format!("DOI: {}", doi)]);
            }

            if let Some(pdf) = pdf_url {
                result.metadata.img_src = Some(pdf);
            }

            if let Some(html) = html_url {
                result.metadata.template = Some(html);
            }

            if let Some(tags) = tags {
                if !tags.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.extend(tags);
                    } else {
                        result.metadata.tags = Some(tags);
                    }
                }
            }

            if let Some(journal) = journal {
                if !journal.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.insert(0, format!("Journal: {}", journal));
                    } else {
                        result.metadata.tags = Some(vec![format!("Journal: {}", journal)]);
                    }
                }
            }

            if let Some(publisher) = publisher {
                if !publisher.is_empty() {
                    if let Some(existing) = result.metadata.tags.as_mut() {
                        existing.push(format!("Publisher: {}", publisher));
                    } else {
                        result.metadata.tags = Some(vec![format!("Publisher: {}", publisher)]);
                    }
                }
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for OpenAlex {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for OpenAlex {
    fn name(&self) -> &str {
        "openalex"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://openalex.org/")
            .official_api(true)
            .results_format("JSON")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "scientific publications", "academic"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn supports_time_range(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        use serde_json::json;

        let mut query_params = HashMap::new();

        // Build search query
        let query_body = json!({
            "filter": format!("search={}", params.query),
            "page": params.pageno,
            "per-page": 10,
            "sort": "relevance_score:desc",
        });

        query_params.insert("query".to_string(), query_body.to_string());

        // Add language filter if not default
        if params.lang != "all" && !params.lang.is_empty() {
            let iso2 = params.lang.split('-').next().unwrap_or(&params.lang);
            query_params.insert("lang_filter".to_string(), format!("language:{}", iso2));
        }

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
    fn test_openalex_request() {
        let openalex = OpenAlex::new();
        let params = RequestParams::new("artificial intelligence");
        let request = openalex.request(&params).unwrap();

        assert!(request.url.contains("openalex.org"));
        assert!(request.params.contains_key("query"));
    }
}
