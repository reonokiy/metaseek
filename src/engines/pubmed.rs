//! PubMed search engine implementation
//!
//! Search biomedical literature using the PubMed database.
//! Reference: python-engines/pubmed.py

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// PubMed biomedical literature search engine
pub struct PubMed {
    base_url: String,
}

impl PubMed {
    pub fn new() -> Self {
        Self {
            base_url: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi".to_string(),
        }
    }

    /// Parse PubMed XML response
    fn parse_results(&self, xml_content: &str) -> Vec<Result> {
        let mut results = Vec::new();
        let mut position = 1u32;

        // Simple XML parsing for PubMed articles
        // Look for PubmedArticle elements
        for article_block in xml_content.split("<PubmedArticle>") {
            if !article_block.contains("</PubmedArticle>") {
                continue;
            }

            let mut title = String::new();
            let mut url = String::new();
            let mut content = String::new();
            let mut authors: Vec<String> = Vec::new();
            let mut journal: Option<String> = None;
            let mut issn: Option<String> = None;
            let mut doi: Option<String> = None;
            let mut published_date: Option<String> = None;

            // Extract title
            if let Some(title_start) = article_block.find("<ArticleTitle>") {
                let content_start = title_start + 14; // length of <ArticleTitle>
                if let Some(title_end) = article_block[content_start..].find("</ArticleTitle>") {
                    title = article_block[content_start..content_start + title_end]
                        .trim()
                        .to_string();
                }
            }

            if title.is_empty() {
                continue;
            }

            // Extract PMID for URL
            if let Some(pmid_start) = article_block.find("<PMID>") {
                let content_start = pmid_start + 6; // length of <PMID>
                if let Some(pmid_end) = article_block[content_start..].find("</PMID>") {
                    let pmid = article_block[content_start..content_start + pmid_end]
                        .trim()
                        .to_string();
                    url = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", pmid);
                }
            }

            // Extract authors
            for author_block in article_block.split("<Author>") {
                if author_block.is_empty() {
                    continue;
                }

                let first_name = author_block
                    .split("<ForeName>")
                    .nth(1)
                    .and_then(|s| s.split("</ForeName>").next())
                    .map(|s| s.trim());

                let last_name = author_block
                    .split("<LastName>")
                    .nth(1)
                    .and_then(|s| s.split("</LastName>").next())
                    .map(|s| s.trim());

                if let (Some(f), Some(l)) = (first_name, last_name) {
                    let author_name = format!("{} {}", f, l);
                    authors.push(author_name);
                }
            }

            // Extract journal title
            if let Some(journal_start) = article_block.find("<JournalTitle>") {
                let content_start = journal_start + 14;
                if let Some(journal_end) = article_block[content_start..].find("</JournalTitle>") {
                    journal = Some(
                        article_block[content_start..content_start + journal_end]
                            .trim()
                            .to_string(),
                    );
                }
            }

            // Extract ISSN
            if let Some(issn_start) = article_block.find("<ISSN>") {
                let content_start = issn_start + 6;
                if let Some(issn_end) = article_block[content_start..].find("</ISSN>") {
                    issn = Some(
                        article_block[content_start..content_start + issn_end]
                            .trim()
                            .to_string(),
                    );
                }
            }

            // Extract DOI
            if let Some(doi_start) = article_block.find("<ELocationID") {
                if article_block[doi_start..].contains("EIdType='doi'") {
                    let content_start =
                        doi_start + article_block[doi_start..].find(">").unwrap_or(0) + 1;
                    if let Some(doi_end) = article_block[content_start..].find("</ELocationID>") {
                        doi = Some(
                            article_block[content_start..content_start + doi_end]
                                .trim()
                                .to_string(),
                        );
                    }
                }
            }

            // Extract abstract/content
            if let Some(abstract_start) = article_block.find("<AbstractText") {
                let content_start =
                    abstract_start + article_block[abstract_start..].find(">").unwrap_or(0) + 1;
                if let Some(content_end) = article_block[content_start..].find("</AbstractText>") {
                    content = article_block[content_start..content_start + content_end]
                        .trim()
                        .to_string();
                }
            }

            // Extract publication date
            let date_parts: Vec<&str> = article_block
                .split("<DateCreated>")
                .nth(1)
                .unwrap_or("")
                .split("><")
                .collect();

            let year = date_parts
                .iter()
                .find(|s| s.contains("Year"))
                .and_then(|s| {
                    s.split("<Year>")
                        .nth(1)
                        .and_then(|s| s.split("</Year>").next())
                })
                .map(|s| s.to_string());

            let month = date_parts
                .iter()
                .find(|s| s.contains("Month"))
                .and_then(|s| {
                    s.split("<Month>")
                        .nth(1)
                        .and_then(|s| s.split("</Month>").next())
                })
                .map(|s| s.to_string());

            let day = date_parts
                .iter()
                .find(|s| s.contains("Day"))
                .and_then(|s| {
                    s.split("<Day>")
                        .nth(1)
                        .and_then(|s| s.split("</Day>").next())
                })
                .map(|s| s.to_string());

            if let (Some(y), Some(m), Some(d)) = (year, month, day) {
                published_date = Some(format!("{}-{}-{}", y, m, d));
            }

            // Create result
            let mut result = Result::new(url.clone(), title, self.name().to_string());
            result.result_type = ResultType::Paper;
            result = result.with_position(position);

            if !content.is_empty() {
                result = result.with_content(content);
            }

            if !authors.is_empty() {
                result.metadata.author = Some(authors.join(", "));
            }

            if let Some(j) = journal {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("Journal: {}", j));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("Journal: {}", j)]);
                }
            }

            if let Some(issn_str) = issn {
                if !issn_str.is_empty() {
                    if let Some(tags) = result.metadata.tags {
                        let mut new_tags = tags;
                        new_tags.push(format!("ISSN: {}", issn_str));
                        result.metadata.tags = Some(new_tags);
                    } else {
                        result.metadata.tags = Some(vec![format!("ISSN: {}", issn_str)]);
                    }
                }
            }

            if let Some(doi_str) = doi {
                if let Some(tags) = result.metadata.tags {
                    let mut new_tags = tags;
                    new_tags.push(format!("DOI: {}", doi_str));
                    result.metadata.tags = Some(new_tags);
                } else {
                    result.metadata.tags = Some(vec![format!("DOI: {}", doi_str)]);
                }
            }

            if let Some(date) = published_date {
                result.metadata.published_date = Some(date);
            }

            results.push(result);
            position += 1;
        }

        results
    }
}

impl Default for PubMed {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for PubMed {
    fn name(&self) -> &str {
        "pubmed"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://pubmed.ncbi.nlm.nih.gov/")
            .official_api(true)
            .results_format("XML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["science", "scientific publications", "medical"]
    }

    fn supports_paging(&self) -> bool {
        true
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        // PubMed uses eutils API
        // First we do a search to get PMIDs, then fetch details
        // For simplicity, we'll search directly

        let mut query_params = HashMap::new();
        query_params.insert("db".to_string(), "pubmed".to_string());
        query_params.insert("term".to_string(), params.query.clone());
        query_params.insert(
            "retstart".to_string(),
            ((params.pageno - 1) * 10).to_string(),
        );
        query_params.insert("hits".to_string(), "10".to_string());

        let mut request =
            EngineRequest::get("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi");
        request.params = query_params;

        Ok(request)
    }

    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults> {
        if !response.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status));
        }

        // Parse XML response
        let results = self.parse_results(&response.text);

        Ok(EngineResults::with_results(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubmed_request() {
        let pubmed = PubMed::new();
        let params = RequestParams::new("cancer research");
        let request = pubmed.request(&params).unwrap();

        assert!(request.url.contains("eutils"));
        assert!(request.url.contains("ncbi.nlm.nih.gov"));
        assert!(request.params.contains_key("term"));
        assert!(request.params.contains_key("db"));
        assert_eq!(request.params.get("db"), Some(&"pubmed".to_string()));
    }

    #[test]
    fn test_pubmed_categories() {
        let pubmed = PubMed::new();
        let categories = pubmed.categories();

        assert!(categories.contains(&"science"));
        assert!(categories.contains(&"scientific publications"));
        assert!(categories.contains(&"medical"));
    }

    #[test]
    fn test_pubmed_paging() {
        let pubmed = PubMed::new();
        assert!(pubmed.supports_paging());
    }
}
