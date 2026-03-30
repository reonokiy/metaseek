//! IMDb (Internet Movie Database) search engine implementation
//!
//! Search for movies, TV shows, actors, and entertainment content.
//! Reference: python-engines/imdb.py

use super::traits::*;
use crate::results::{Result, ResultType};
use anyhow::Result as AnyhowResult;
use std::collections::HashMap;

/// IMDb search engine for movies and entertainment
pub struct Imdb {}

impl Imdb {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse IMDb search results JSON
    fn parse_results(&self, json_data: &serde_json::Value) -> Vec<Result> {
        let mut results = Vec::new();

        let suggestions = json_data
            .get("d")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut position = 1u32;

        // Category mapping from Python reference
        let search_categories: HashMap<&str, &str> = HashMap::from([
            ("nm", "name"),
            ("tt", "title"),
            ("kw", "keyword"),
            ("co", "company"),
            ("ep", "episode"),
        ]);

        for item in suggestions {
            let entry_id = item.get("id").and_then(|i| i.as_str()).unwrap_or_default();

            if entry_id.is_empty() {
                continue;
            }

            // Get category
            let category = search_categories.get(&entry_id[..2]).copied();
            if category.is_none() {
                continue;
            }

            let category = category.unwrap();

            // Build URL
            let url = format!("https://imdb.com/{}/{}", category, entry_id);

            // Get title
            let title = item
                .get("l")
                .and_then(|l| l.as_str())
                .unwrap_or_default()
                .to_string();

            // Add year if available
            let year = item
                .get("y")
                .and_then(|y| y.as_str())
                .map(|y| y.to_string());

            let full_title = if let Some(ref y) = year {
                format!("{} ({})", title, y)
            } else {
                title
            };

            // Get subtitle/description
            let subtitle = item
                .get("s")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            // Get rank
            let rank = item.get("rank").and_then(|r| r.as_u64());

            // Extract image thumbnail
            let image_url = item
                .get("i")
                .and_then(|i| i.as_object())
                .and_then(|img| img.get("imageUrl"))
                .and_then(|url| url.as_str());

            let thumbnail = image_url.map(|img_url| {
                // Process image URL to get thumbnail following Python logic
                // The magic recipe: QL75_UX280_CR0,0,280,414_
                if img_url.ends_with(".jpg") {
                    let base = img_url.strip_suffix(".jpg").unwrap_or(img_url);
                    format!("{}/QL75_UX280_CR0,0,280,414_.jpg", base)
                } else {
                    img_url.to_string()
                }
            });

            // Build content/description
            let mut content_parts = Vec::new();
            if let Some(rank) = rank {
                content_parts.push(format!("#{} ", rank));
            }
            if let Some(year) = year {
                content_parts.push(format!("{} - ", year));
            }
            if let Some(subtitle) = subtitle {
                content_parts.push(subtitle);
            }

            let content = if content_parts.is_empty() {
                None
            } else {
                Some(content_parts.join(""))
            };

            // Create result
            let mut result = Result::new(url, full_title, self.name().to_string());
            result.result_type = ResultType::Default;

            if let Some(content) = content {
                result = result.with_content(content);
            }

            if let Some(thumb) = thumbnail {
                result.metadata.img_src = Some(thumb);
            }

            result = result.with_position(position);
            position += 1;

            results.push(result);
        }

        results
    }
}

impl Default for Imdb {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Imdb {
    fn name(&self) -> &str {
        "imdb"
    }

    fn about(&self) -> EngineAbout {
        EngineAbout::new()
            .website("https://imdb.com/")
            .official_api(false)
            .results_format("HTML")
    }

    fn categories(&self) -> Vec<&str> {
        vec!["movies"]
    }

    fn supports_paging(&self) -> bool {
        false
    }

    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest> {
        let query = params.query.to_lowercase().replace(' ', "_");

        // Extract first letter for URL
        let letter = query.chars().next().unwrap_or('a');
        let letter_url = letter.to_string();

        let query_url = format!(
            "https://v2.sg.media-imdb.com/suggestion/{}/{}.json",
            letter_url, query
        );

        let request = EngineRequest::get(query_url);

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
    fn test_imdb_request() {
        let imdb = Imdb::new();
        let params = RequestParams::new("matrix movie");
        let request = imdb.request(&params).unwrap();

        assert!(request.url.contains("media-imdb.com"));
        assert!(request.url.contains("suggestion"));
    }

    #[test]
    fn test_imdb_categories() {
        let imdb = Imdb::new();
        let categories = imdb.categories();

        assert!(categories.contains(&"movies"));
    }

    #[test]
    fn test_imdb_no_paging() {
        let imdb = Imdb::new();
        assert!(!imdb.supports_paging());
    }
}
