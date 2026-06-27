//! Query parsing module
//!
//! Handles parsing of user queries including special syntax like:
//! - Language specifiers: `:en`, `:de`
//! - Category/engine bangs: `!images`, `!google`, `!tor`
//! - External bangs: `!g`, `!yt`
//! - Timeout specifiers: `<3`
//! - Safe search toggle: `!safesearch`
//! - Time range: `!day`, `!week`, `!month`, `!year`

use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::engines::registry::get_category_map;

/// Parsed search query with extracted special syntax
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    /// The cleaned search query (without special syntax)
    pub query: String,
    /// Original raw query
    pub raw_query: String,
    /// Detected language codes
    pub languages: Vec<String>,
    /// Requested categories (resolved to engine lists later)
    pub categories: Vec<String>,
    /// Specific engines requested
    pub engines: Vec<String>,
    /// External bang (e.g., !g for Google redirect)
    pub external_bang: Option<String>,
    /// Custom timeout in seconds
    pub timeout: Option<f64>,
    /// Safe search override (None = use default)
    pub safesearch: Option<u8>,
    /// Time range filter
    pub time_range: Option<TimeRange>,
    /// Page number
    pub pageno: u32,
    /// Redirect to first result
    pub redirect_to_first: bool,
    /// If true, ignore default engines/categories and only use specified ones
    pub specific: bool,
}

impl ParsedQuery {
    /// Parse a raw query string
    pub fn parse(raw: &str) -> Self {
        let category_map = get_category_map();
        
        let mut query = raw.to_string();
        let mut languages = Vec::new();
        let mut categories = Vec::new();
        let mut engines: Vec<String> = Vec::new();
        let mut external_bang = None;
        let mut timeout = None;
        let mut safesearch = None;
        let mut time_range = None;
        let mut redirect_to_first = false;
        let mut specific = false;

        // Parse language specifiers :xx or :xx-XX
        let lang_re = Regex::new(r":([a-z]{2}(?:-[A-Z]{2})?)(?:\s|$)").unwrap();
        for cap in lang_re.captures_iter(&query) {
            languages.push(cap[1].to_string());
        }
        query = lang_re.replace_all(&query, " ").to_string();

        // Parse timeout <N or <Nms
        let timeout_re = Regex::new(r"<(\d+(?:\.\d+)?)(ms)?(?:\s|$)").unwrap();
        if let Some(cap) = timeout_re.captures(&query) {
            let value: f64 = cap[1].parse().unwrap_or(5.0);
            timeout = Some(if cap.get(2).is_some() {
                value / 1000.0
            } else {
                value
            });
        }
        query = timeout_re.replace_all(&query, " ").to_string();

        // Parse safesearch toggle
        if query.contains("!safesearch") {
            safesearch = Some(2);
            query = query.replace("!safesearch", " ");
        }
        if query.contains("!nosafesearch") {
            safesearch = Some(0);
            query = query.replace("!nosafesearch", " ");
        }

        // Parse time range
        let time_ranges = [
            ("!day", TimeRange::Day),
            ("!week", TimeRange::Week),
            ("!month", TimeRange::Month),
            ("!year", TimeRange::Year),
        ];
        for (pattern, range) in time_ranges {
            if query.contains(pattern) {
                time_range = Some(range);
                query = query.replace(pattern, " ");
                break;
            }
        }

        // Parse redirect to first result
        // Handle !! anywhere in the query
        if query.contains("!!") {
            redirect_to_first = true;
            query = query.replace("!!", " ");
        }
        // Also handle ! at the start for single ! redirect (if that's a thing, but usually it's !!)
        // The original logic for ! at start was for ! followed by space, which is different.
        // Let's keep the original logic for ! at start if it was meant for something else.
        // Actually, the original logic:
        // if query.starts_with('!') && query.chars().nth(1).map(|c| c == ' ').unwrap_or(true) {
        //     redirect_to_first = true;
        //     query = query.trim_start_matches('!').to_string();
        // }
        // This seems to handle a single ! at the start followed by space, which is not standard.
        // Standard is !! for redirect. Let's remove the single ! logic if it's not needed.
        // But to be safe, let's keep it and just add the !! handling.
        
        // Re-check single ! at start (original logic)
        if query.starts_with('!') && query.chars().nth(1).map(|c| c == ' ').unwrap_or(true) {
            redirect_to_first = true;
            query = query.trim_start_matches('!').to_string();
        }

        // --- DYNAMIC CATEGORY/ENGINE PARSING ---
        let bang_re = Regex::new(r"!(\w+)(?:\s|$)").unwrap();
        let mut processed_bangs = Vec::new();

        for cap in bang_re.captures_iter(&query) {
            let bang = cap[1].to_lowercase();
            processed_bangs.push(bang.clone());
        }

        for bang in &processed_bangs {
            specific = true;

            if Self::is_external_bang(bang) {
                external_bang = Some(bang.clone());
                continue;
            }

            // 1. Check if it's a category in the generated map
            if let Some(engine_list) = category_map.get(bang) {
                for engine in engine_list {
                    if !engines.contains(engine) {
                        engines.push(engine.clone());
                    }
                }
                categories.push(bang.clone());
                continue;
            }

            // 2. Check if it's a known engine name (fallback)
            engines.push(bang.clone());
        }

        // Remove processed bangs from query
        for bang in &processed_bangs {
            query = query.replace(&format!("!{}", bang), " ");
        }

        // Clean up whitespace
        query = query.split_whitespace().collect::<Vec<_>>().join(" ");

        Self {
            query,
            raw_query: raw.to_string(),
            languages,
            categories,
            engines,
            external_bang,
            timeout,
            safesearch,
            time_range,
            pageno: 1,
            redirect_to_first,
            specific,
        }
    }

    /// Check if a bang should redirect to external site
    fn is_external_bang(bang: &str) -> bool {
        let external = ["g", "yt", "w", "wa", "amazon", "imdb"];
        external.contains(&bang)
    }

    /// Check if query is empty after parsing
    pub fn is_empty(&self) -> bool {
        self.query.trim().is_empty()
    }

    /// Get the effective categories (requested or default)
    pub fn effective_categories(&self, default: &[String]) -> Vec<String> {
        if self.categories.is_empty() {
            default.to_vec()
        } else {
            self.categories.clone()
        }
    }
}

/// Time range filter for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimeRange {
    Day,
    Week,
    Month,
    Year,
}

impl TimeRange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

impl std::fmt::Display for TimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query() {
        let parsed = ParsedQuery::parse("hello world");
        assert_eq!(parsed.query, "hello world");
        assert!(parsed.languages.is_empty());
        assert!(!parsed.specific);
    }

    #[test]
    fn test_language_parsing() {
        let parsed = ParsedQuery::parse("hello :en world");
        assert_eq!(parsed.query, "hello world");
        assert_eq!(parsed.languages, vec!["en"]);
    }

    #[test]
    fn test_timeout_parsing() {
        let parsed = ParsedQuery::parse("hello <3 world");
        assert_eq!(parsed.query, "hello world");
        assert_eq!(parsed.timeout, Some(3.0));
    }

    #[test]
    fn test_category_bang() {
        let parsed = ParsedQuery::parse("rust tutorial !tor");
        assert_eq!(parsed.query, "rust tutorial");
        assert!(parsed.specific);
        // Check if ahmia is in engines (if tor category exists)
        if parsed.categories.contains(&"tor".to_string()) {
            assert!(parsed.engines.contains(&"ahmia".to_string()));
        }
    }

    #[test]
    fn test_engine_bang() {
        let parsed = ParsedQuery::parse("rust !google");
        assert_eq!(parsed.query, "rust");
        assert!(parsed.engines.contains(&"google".to_string()));
        assert!(parsed.specific);
    }

    #[test]
    fn test_time_range() {
        let parsed = ParsedQuery::parse("news !week");
        assert_eq!(parsed.query, "news");
        assert_eq!(parsed.time_range, Some(TimeRange::Week));
    }

    #[test]
    fn test_safesearch() {
        let parsed = ParsedQuery::parse("query !safesearch");
        assert_eq!(parsed.safesearch, Some(2));
    }

    #[test]
    fn test_multiple_bangs() {
        let parsed = ParsedQuery::parse("!images !tor test");
        assert_eq!(parsed.query, "test");
        assert!(parsed.specific);
        // Should contain engines from both categories if they exist
        if parsed.categories.contains(&"images".to_string()) {
            assert!(parsed.engines.iter().any(|e| e.contains("google_images") || e.contains("bing_images")));
        }
        if parsed.categories.contains(&"tor".to_string()) {
            assert!(parsed.engines.contains(&"ahmia".to_string()));
        }
    }

    #[test]
    fn test_external_bang() {
        let parsed = ParsedQuery::parse("cat !g");
        assert_eq!(parsed.query, "cat");
        assert_eq!(parsed.external_bang, Some("g".to_string()));
        assert!(parsed.specific);
    }

    #[test]
    fn test_redirect_first() {
        let parsed = ParsedQuery::parse("!! weather");
        assert_eq!(parsed.query, "weather");
        assert!(parsed.redirect_to_first);
    }

    #[test]
    fn test_category_bang_multiple() {
        let parsed = ParsedQuery::parse("test !images !science");
        assert_eq!(parsed.query, "test");
        assert!(parsed.specific);
        assert!(parsed.engines.contains(&"google_images".to_string()) || parsed.engines.contains(&"bing_images".to_string()));
        assert!(parsed.engines.contains(&"arxiv".to_string()) || parsed.engines.contains(&"pubmed".to_string()));
        assert!(parsed.categories.contains(&"images".to_string()));
        assert!(parsed.categories.contains(&"science".to_string()));
    }

    #[test]
    fn test_nosafesearch() {
        let parsed = ParsedQuery::parse("query !nosafesearch");
        assert_eq!(parsed.query, "query");
        assert_eq!(parsed.safesearch, Some(0));
        assert!(!parsed.specific);
    }

    #[test]
    fn test_unknown_category() {
        let parsed = ParsedQuery::parse("test !unknown_category_xyz");
        assert_eq!(parsed.query, "test");
        assert!(parsed.specific);
        assert!(parsed.engines.contains(&"unknown_category_xyz".to_string()));
    }

    #[test]
    fn test_empty_query_after_parsing() {
        let parsed = ParsedQuery::parse("!tor");
        assert!(parsed.is_empty());
        assert!(parsed.specific);
        assert!(parsed.engines.contains(&"ahmia".to_string()));
    }

    #[test]
    fn test_timeout_parsing_ms() {
        let parsed = ParsedQuery::parse("hello <850ms world");
        assert_eq!(parsed.query, "hello world");
        assert_eq!(parsed.timeout, Some(0.85));
        assert!(!parsed.specific);
    }

    #[test]
    fn test_redirect_first_with_bang() {
        let parsed = ParsedQuery::parse("!google !! weather");
        assert_eq!(parsed.query, "weather");
        assert!(parsed.redirect_to_first);
        assert!(parsed.specific);
        assert!(parsed.engines.contains(&"google".to_string()));
    }

    #[test]
    fn test_multiple_features() {
        let parsed = ParsedQuery::parse(":en !tor <5 !safesearch test");
        assert_eq!(parsed.query, "test");
        assert_eq!(parsed.languages, vec!["en"]);
        assert!(parsed.engines.contains(&"ahmia".to_string()));
        assert_eq!(parsed.timeout, Some(5.0));
        assert_eq!(parsed.safesearch, Some(2));
        assert!(parsed.specific);
    }
}