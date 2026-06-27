//! Test to verify JSON output format matches upstream SearXNG Python format

use crate::web::handlers::{ParsedUrlArray, ResultResponse, SearchResponse};
use serde_json::json;

#[test]
fn test_parsed_url_array_serialization() {
    // Test that ParsedUrlArray serializes as an array, not an object
    let parsed_url = ParsedUrlArray(vec![
        "https".to_string(),
        "example.com".to_string(),
        "/path".to_string(),
        "query=1".to_string(),
        "fragment".to_string(),
        "".to_string(),
    ]);

    let serialized = serde_json::to_string(&parsed_url).unwrap();
    assert_eq!(serialized, r#"["https","example.com","/path","query=1","fragment",""]"#);
}

#[test]
fn test_result_response_has_all_required_fields() {
    // Verify that ResultResponse includes all fields from upstream Python format
    let result = ResultResponse {
        url: "https://example.com".to_string(),
        title: "Example".to_string(),
        content: Some("Content".to_string()),
        engine: "duckduckgo".to_string(),
        engines: vec!["duckduckgo".to_string()],
        positions: vec![1],
        score: 1.0,
        category: Some("general".to_string()),
        template: Some("default.html".to_string()),
        img_src: None,
        thumbnail: None,
        priority: Some("".to_string()),
        published_date: None,
        author: None,
        file_type: None,
        file_size: None,
        duration: None,
        views: None,
        iframe_src: None,
        audio_src: None,
        is_official: false,
        version: None,
        license: None,
        tags: None,
        source_code: None,
        homepage: None,
        documentation: None,
        last_update: None,
        architecture: None,
        package_name: None,
        stars: None,
        forks: None,
        verified: None,
        severity: None,
        cvss_score: None,
        modified_date: None,
        archived: None,
        mirror: None,
        trending_score: None,
        library: None,
        pipeline: None,
        private: None,
        disabled: None,
        result_type: "default".to_string(),
        parsed_url: ParsedUrlArray(vec![
            "https".to_string(),
            "example.com".to_string(),
            "/".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]),
    };

    let serialized = serde_json::to_value(&result).unwrap();
    
    // Verify all required fields are present
    assert!(serialized.get("url").is_some());
    assert!(serialized.get("title").is_some());
    assert!(serialized.get("content").is_some());
    assert!(serialized.get("engine").is_some());
    assert!(serialized.get("engines").is_some());
    assert!(serialized.get("positions").is_some());
    assert!(serialized.get("score").is_some());
    assert!(serialized.get("category").is_some());
    assert!(serialized.get("template").is_some());
    assert!(serialized.get("img_src").is_some());
    assert!(serialized.get("thumbnail").is_some());
    assert!(serialized.get("priority").is_some());
    assert!(serialized.get("parsed_url").is_some());
    
    // Verify parsed_url is an array
    let parsed_url = serialized.get("parsed_url").unwrap();
    assert!(parsed_url.is_array());
    assert_eq!(parsed_url.as_array().unwrap().len(), 6);
}

#[test]
fn test_search_response_structure() {
    // Verify that SearchResponse matches upstream Python format
    let response = SearchResponse {
        query: "test".to_string(),
        number_of_results: 10,
        results: vec![],
        answers: vec![],
        suggestions: vec![],
        corrections: vec![],
        infoboxes: vec![],
        unresponsive_engines: vec![],
        engine_errors: vec![],
    };

    let serialized = serde_json::to_value(&response).unwrap();
    
    // Verify all required fields are present
    assert!(serialized.get("query").is_some());
    assert!(serialized.get("number_of_results").is_some());
    assert!(serialized.get("results").is_some());
    assert!(serialized.get("answers").is_some());
    assert!(serialized.get("suggestions").is_some());
    assert!(serialized.get("corrections").is_some());
    assert!(serialized.get("infoboxes").is_some());
    assert!(serialized.get("unresponsive_engines").is_some());
    assert!(serialized.get("engine_errors").is_some());
}