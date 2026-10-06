//! HTTP request handlers

use super::state::AppState;
use crate::engines::EngineRegistry;
use crate::mcp::{McpError, McpRequest, McpResponse};
use crate::query::{ParsedQuery, TimeRange};
use crate::search::{EngineRef, SearchQuery};
use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tera::Context;
use url::Url;

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query
    pub q: Option<String>,
    /// Categories (comma-separated)
    pub categories: Option<String>,
    /// Engines (comma-separated)
    pub engines: Option<String>,
    /// Language
    pub language: Option<String>,
    /// Time range
    pub time_range: Option<String>,
    /// Safe search level
    pub safesearch: Option<u8>,
    /// Page number
    pub pageno: Option<u32>,
    /// Output format
    pub format: Option<String>,
}

/// Search results response for JSON format
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub number_of_results: usize,
    pub results: Vec<ResultResponse>,
    pub answers: Vec<AnswerResponse>,
    pub suggestions: Vec<String>,
    pub corrections: Vec<String>,
    pub infoboxes: Vec<serde_json::Value>,
    pub unresponsive_engines: Vec<UnresponsiveEngineResponse>,
    pub engine_errors: Vec<EngineError>,
}

/// Engine error information
#[derive(Debug, Serialize)]
pub struct EngineError {
    pub engine: String,
    pub error: String,
    pub error_type: ErrorType,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    ApiKeyRequired,
    RateLimited,
    NetworkError,
    ParseError,
    Timeout,
    Other,
}

/// Unresponsive engine information
#[derive(Debug, Serialize)]
pub struct UnresponsiveEngineResponse {
    pub engine: String,
    pub error: String,
}

/// A single search result in JSON response
#[derive(Debug, Serialize)]
pub struct ResultResponse {
    pub url: String,
    pub title: String,
    pub content: Option<String>,
    pub engine: String,
    pub engines: Vec<String>,
    pub positions: Vec<u32>,
    pub score: f64,
    pub category: Option<String>,
    pub template: Option<String>,
    pub img_src: Option<String>,
    pub thumbnail: Option<String>,
    pub priority: Option<String>,
    pub published_date: Option<String>,
    #[serde(rename = "publishedDate")]
    pub published_date_compat: Option<String>,
    pub author: Option<String>,
    pub file_type: Option<String>,
    pub file_size: Option<String>,
    pub duration: Option<String>,
    pub views: Option<u64>,
    pub iframe_src: Option<String>,
    pub audio_src: Option<String>,
    pub is_official: bool,
    pub version: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_code: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub last_update: Option<String>,
    pub architecture: Option<String>,
    pub package_name: Option<String>,
    pub stars: Option<u64>,
    pub forks: Option<u64>,
    pub verified: Option<bool>,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub modified_date: Option<String>,
    pub archived: Option<bool>,
    pub mirror: Option<bool>,
    pub trending_score: Option<f64>,
    pub library: Option<String>,
    pub pipeline: Option<String>,
    pub private: Option<bool>,
    pub disabled: Option<bool>,
    pub result_type: String,
    pub parsed_url: ParsedUrlArray,
}

/// Parsed URL as array [scheme, domain, path, query, fragment, extra]
#[derive(Debug, Serialize)]
pub struct ParsedUrlArray(pub Vec<String>);

impl From<&Url> for ParsedUrlArray {
    fn from(url: &Url) -> Self {
        Self(vec![
            url.scheme().to_string(),
            url.host_str().unwrap_or("").to_string(),
            url.path().to_string(),
            url.query().unwrap_or("").to_string(),
            url.fragment().unwrap_or("").to_string(),
            "".to_string(),
        ])
    }
}

/// Answer result in JSON response
#[derive(Debug, Serialize)]
pub struct AnswerResponse {
    pub answer: String,
    pub engine: String,
    pub url: Option<String>,
}

/// Home page handler
pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let mut ctx = Context::new();
    ctx.insert("instance_name", state.instance_name());
    ctx.insert("version", crate::VERSION);
    if let Some(name) = state.branding_name() {
        ctx.insert("brand_name", name);
    }
    if let Some(tagline) = state.branding_tagline() {
        ctx.insert("brand_tagline", tagline);
    }
    if let Some(color) = state.branding_accent_color() {
        ctx.insert("accent_color", color);
    }
    if let Some(url) = state.branding.logo.as_ref() {
        ctx.insert("logo_url", url);
    }
    if let Some(data_uri) = state.branding.logo_data_uri.as_ref() {
        ctx.insert("logo_data_uri", data_uri);
    }
    ctx.insert(
        "categories",
        &["general", "images", "videos", "news", "it", "science"],
    );

    match state.templates.render_with_context("index.html", &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}

/// Normalize comma-separated selectors; empty values behave like omission.
fn selectors(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Explicit HTTP selectors override query bangs. Engines take precedence over
/// categories for selection; a matching category supplies their display context.
fn build_search_query(
    params: &SearchParams,
    registry: &EngineRegistry,
) -> Result<SearchQuery, String> {
    let raw = params.q.as_deref().unwrap_or_default();
    if raw.trim().is_empty() {
        return Err("q must not be empty".into());
    }
    let parsed = ParsedQuery::parse(raw);
    let http_engines = selectors(params.engines.as_deref());
    let http_categories = selectors(params.categories.as_deref());
    let has_http_selector = !http_engines.is_empty() || !http_categories.is_empty();
    let categories = if !http_categories.is_empty() {
        http_categories
    } else if !has_http_selector && !parsed.categories.is_empty() {
        parsed.categories.clone()
    } else {
        vec![]
    };
    for category in &categories {
        if registry.get_by_category(category).is_empty() {
            return Err(format!("Unknown or unavailable category: {category}"));
        }
    }
    // Category bangs expand from a static catalog; resolve them against the live
    // registry so disabled/unimplemented engines are never selected.
    let engines = if !http_engines.is_empty() {
        http_engines
    } else if !has_http_selector && parsed.categories.is_empty() {
        parsed.engines.clone()
    } else {
        vec![]
    };
    let mut refs = Vec::new();
    let mut seen = HashSet::new();
    if !engines.is_empty() {
        for name in engines {
            let engine = registry
                .get(&name)
                .ok_or_else(|| format!("Unknown or unavailable engine: {name}"))?;
            let category = categories
                .iter()
                .find(|c| engine.categories().contains(&c.as_str()))
                .map(String::as_str)
                .unwrap_or_else(|| engine.categories().first().copied().unwrap_or("general"));
            if seen.insert(name.clone()) {
                refs.push(EngineRef::new(name, category));
            }
        }
    } else {
        let effective = if categories.is_empty() {
            vec!["general".to_string()]
        } else {
            categories
        };
        for category in effective {
            for engine in registry.get_by_category(&category) {
                if seen.insert(engine.name().to_string()) {
                    refs.push(EngineRef::new(engine.name(), &category));
                }
            }
        }
    }
    let mut query = SearchQuery::from_parsed(parsed, vec![]);
    query.engine_refs = refs;
    query.pageno = params.pageno.unwrap_or(query.pageno);
    if !(1..=1000).contains(&query.pageno) {
        return Err("pageno must be between 1 and 1000".into());
    }
    if let Some(level) = params.safesearch {
        query.safesearch = level;
    }
    if query.safesearch > 2 {
        return Err("safesearch must be 0, 1 or 2".into());
    }
    if let Some(language) = &params.language {
        if !language.trim().is_empty() {
            query.lang = language.trim().to_string();
        }
    }
    if let Some(range) = &params.time_range {
        query.time_range = match range.trim() {
            "" | "anytime" => None,
            "day" => Some(TimeRange::Day),
            "week" => Some(TimeRange::Week),
            "month" => Some(TimeRange::Month),
            "year" => Some(TimeRange::Year),
            _ => return Err("time_range must be day, week, month, year or anytime".into()),
        };
    }
    if query.is_empty() {
        return Err("q must contain search terms".into());
    }
    Ok(query)
}

fn api_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({"error": {"message": message}})),
    )
        .into_response()
}

fn http_url(value: &str) -> bool {
    Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

/// Search handler. Extractor failures also return JSON rather than plain text.
pub async fn search(
    State(state): State<AppState>,
    params: Result<Query<SearchParams>, QueryRejection>,
) -> Response {
    let Query(params) = match params {
        Ok(params) => params,
        Err(_) => return api_error(StatusCode::BAD_REQUEST, "Invalid search parameter type"),
    };
    if !matches!(
        params.format.as_deref(),
        None | Some("html" | "json" | "csv")
    ) {
        return api_error(StatusCode::BAD_REQUEST, "format must be html, json or csv");
    }
    let json_mode = params.format.as_deref() == Some("json");
    let raw_query = params.q.clone().unwrap_or_default();
    if raw_query.trim().is_empty() && !json_mode && params.format.as_deref() != Some("csv") {
        return Redirect::to("/").into_response();
    }
    let mut search_query = match build_search_query(&params, &state.registry) {
        Ok(query) => query,
        Err(message) => return api_error(StatusCode::BAD_REQUEST, &message),
    };
    // An API caller expects JSON, never a redirect to an external HTML page.
    if json_mode {
        if search_query.external_bang.is_some() {
            return api_error(
                StatusCode::BAD_REQUEST,
                "External redirect bangs are not supported with format=json",
            );
        }
        search_query.redirect_to_first = false;
    }
    if search_query.engine_refs.is_empty() && search_query.external_bang.is_none() {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "No search engines available",
        );
    }

    // Execute search
    let results = state.search.execute(&search_query).await;

    // Check for redirect
    if let Some(redirect_url) = results.get_redirect() {
        return Redirect::to(&redirect_url).into_response();
    }

    // Check for redirect to first result
    if search_query.redirect_to_first {
        let ordered = results.get_ordered_results();
        if let Some(first) = ordered.first() {
            return Redirect::to(&first.url).into_response();
        }
    }

    // Log any engine errors for debugging
    for unresponsive in results.get_unresponsive() {
        tracing::warn!(
            "Engine '{}' was unresponsive: {:?}",
            unresponsive.name,
            unresponsive.error
        );
    }

    // Format response based on requested format
    match params.format.as_deref() {
        Some("json") => {
            if results.get_timings().is_empty() && !results.get_unresponsive().is_empty() {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({
                        "error": {"message": "All selected search engines failed"},
                        "query": raw_query,
                        "results": [],
                        "number_of_results": 0,
                        "engine_errors": results.get_unresponsive().iter().map(|e|
                            serde_json::json!({"engine": e.name, "error": e.error.to_string()})
                        ).collect::<Vec<_>>()
                    })),
                )
                    .into_response();
            }
            let ordered: Vec<_> = results
                .get_ordered_results()
                .into_iter()
                .filter(|result| http_url(&result.url))
                .collect();

            // Convert unresponsive engines to detailed error information
            let engine_errors: Vec<EngineError> = results
                .get_unresponsive()
                .iter()
                .map(|ue| EngineError {
                    engine: ue.name.clone(),
                    error: ue.error.to_string(),
                    error_type: match ue.error {
                        crate::results::EngineError::MissingApiKey => ErrorType::ApiKeyRequired,
                        crate::results::EngineError::RateLimited => ErrorType::RateLimited,
                        crate::results::EngineError::NetworkError => ErrorType::NetworkError,
                        crate::results::EngineError::ParseError => ErrorType::ParseError,
                        crate::results::EngineError::Timeout => ErrorType::Timeout,
                        _ => ErrorType::Other,
                    },
                })
                .collect();

            let response = SearchResponse {
                query: raw_query,
                number_of_results: ordered.len(),
                results: ordered
                    .into_iter()
                    .map(|r| {
                        let parsed_url = r.parsed_url.as_ref().map(ParsedUrlArray::from);
                        let result_type_str = match r.result_type {
                            crate::results::ResultType::Default => "default",
                            crate::results::ResultType::Image => "image",
                            crate::results::ResultType::Video => "video",
                            crate::results::ResultType::Map => "map",
                            crate::results::ResultType::News => "news",
                            crate::results::ResultType::Paper => "paper",
                            crate::results::ResultType::File => "file",
                            crate::results::ResultType::Code => "code",
                            crate::results::ResultType::Answer => "answer",
                            crate::results::ResultType::InfoBox => "infobox",
                            crate::results::ResultType::Security => "security",
                            crate::results::ResultType::Corporate => "corporate",
                        };
                        ResultResponse {
                            url: r.url,
                            title: r.title,
                            content: Some(r.content.unwrap_or_default()),
                            engine: r.engine,
                            engines: r.engines.into_iter().collect(),
                            positions: r.positions,
                            score: if r.score.is_finite() { r.score } else { 0.0 },
                            category: Some(
                                r.category
                                    .filter(|c| !c.is_empty())
                                    .unwrap_or_else(|| "general".into()),
                            ),
                            template: r.metadata.template,
                            img_src: r.metadata.img_src.clone(),
                            thumbnail: r
                                .metadata
                                .thumbnail
                                .filter(|u| http_url(u))
                                .or_else(|| r.metadata.img_src.clone().filter(|u| http_url(u))),
                            priority: None, // Could be added if needed
                            published_date_compat: r.metadata.published_date.clone(),
                            published_date: r.metadata.published_date,
                            author: r.metadata.author,
                            file_type: r.metadata.file_type,
                            file_size: r.metadata.file_size,
                            duration: r.metadata.duration,
                            views: r.metadata.views,
                            iframe_src: r.metadata.iframe_src,
                            audio_src: r.metadata.audio_src,
                            is_official: r.metadata.is_official,
                            version: r.metadata.version,
                            license: r.metadata.license,
                            tags: r.metadata.tags,
                            source_code: r.metadata.source_code,
                            homepage: r.metadata.homepage,
                            documentation: r.metadata.documentation,
                            last_update: r.metadata.last_update,
                            architecture: r.metadata.architecture,
                            package_name: r.metadata.package_name,
                            stars: r.metadata.stars,
                            forks: r.metadata.forks,
                            verified: r.metadata.verified,
                            severity: r.metadata.severity,
                            cvss_score: r.metadata.cvss_score,
                            modified_date: r.metadata.modified_date,
                            archived: r.metadata.archived,
                            mirror: r.metadata.mirror,
                            trending_score: r.metadata.trending_score,
                            library: r.metadata.library,
                            pipeline: r.metadata.pipeline,
                            private: r.metadata.private,
                            disabled: r.metadata.disabled,
                            result_type: result_type_str.to_string(),
                            parsed_url: parsed_url.unwrap_or_else(|| {
                                ParsedUrlArray(vec![
                                    "".to_string(),
                                    "".to_string(),
                                    "".to_string(),
                                    "".to_string(),
                                    "".to_string(),
                                    "".to_string(),
                                ])
                            }),
                        }
                    })
                    .collect(),
                answers: results
                    .get_answers()
                    .into_iter()
                    .map(|a| AnswerResponse {
                        answer: a.answer,
                        engine: a.engine,
                        url: a.url,
                    })
                    .collect(),
                suggestions: results
                    .get_suggestions()
                    .into_iter()
                    .map(|s| s.text)
                    .collect(),
                corrections: vec![], // Could be added if needed
                infoboxes: vec![],   // Could be populated if needed
                unresponsive_engines: results
                    .get_unresponsive()
                    .into_iter()
                    .map(|e| UnresponsiveEngineResponse {
                        engine: e.name,
                        error: e.error.to_string(),
                    })
                    .collect(),
                engine_errors,
            };
            Json(response).into_response()
        }
        Some("csv") => {
            let ordered = results.get_ordered_results();
            let mut csv = String::from("title,url,content,engine\n");
            for r in ordered {
                csv.push_str(&format!(
                    "\"{}\",\"{}\",\"{}\",\"{}\"\n",
                    r.title.replace('"', "\"\""),
                    r.url.replace('"', "\"\""),
                    r.content.unwrap_or_default().replace('"', "\"\""),
                    r.engine
                ));
            }
            ([(axum::http::header::CONTENT_TYPE, "text/csv")], csv).into_response()
        }
        _ => {
            // HTML response
            let ordered = results.get_ordered_results();

            let mut ctx = Context::new();
            ctx.insert("instance_name", state.instance_name());
            ctx.insert("version", crate::VERSION);
            if let Some(name) = state.branding_name() {
                ctx.insert("brand_name", name);
            }
            if let Some(tagline) = state.branding_tagline() {
                ctx.insert("brand_tagline", tagline);
            }
            if let Some(color) = state.branding_accent_color() {
                ctx.insert("accent_color", color);
            }
            if let Some(url) = state.branding.logo.as_ref() {
                ctx.insert("logo_url", url);
            }
            if let Some(data_uri) = state.branding.logo_data_uri.as_ref() {
                ctx.insert("logo_data_uri", data_uri);
            }
            ctx.insert("query", &raw_query);
            ctx.insert("results", &ordered);
            ctx.insert("answers", &results.get_answers());
            ctx.insert("suggestions", &results.get_suggestions());
            ctx.insert("infoboxes", &results.get_infoboxes());
            ctx.insert("unresponsive_engines", &results.get_unresponsive());
            ctx.insert("timings", &results.get_timings());
            ctx.insert("result_count", &results.result_count());
            ctx.insert("pageno", &search_query.pageno);
            ctx.insert(
                "categories",
                &["general", "images", "videos", "news", "it", "science"],
            );

            // Add engine errors to context for display
            let engine_errors: Vec<(String, String)> = results
                .get_unresponsive()
                .iter()
                .map(|ue| (ue.name.clone(), ue.error.to_string()))
                .collect();
            ctx.insert("engine_errors", &engine_errors);

            match state.templates.render_with_context("search.html", &ctx) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
                }
            }
        }
    }
}

/// About page handler
pub async fn about(State(state): State<AppState>) -> impl IntoResponse {
    let mut ctx = Context::new();
    ctx.insert("instance_name", state.instance_name());
    ctx.insert("version", crate::VERSION);
    ctx.insert("engines", &state.registry.names());

    match state.templates.render_with_context("about.html", &ctx) {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html("<h1>About</h1><p>Metaseek</p>".to_string())
        }
    }
}

/// Usage page handler
pub async fn usage(State(state): State<AppState>) -> impl IntoResponse {
    let mut ctx = Context::new();
    ctx.insert("instance_name", state.instance_name());
    ctx.insert("version", crate::VERSION);

    match state.templates.render_with_context("usage.html", &ctx) {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html("<h1>Usage Guide</h1><p>Query syntax and API documentation.</p>".to_string())
        }
    }
}

/// Preferences page handler
pub async fn preferences(State(state): State<AppState>) -> impl IntoResponse {
    let mut ctx = Context::new();
    ctx.insert("instance_name", state.instance_name());
    ctx.insert("themes", &state.settings.ui.themes);
    ctx.insert("engines", &state.registry.names());
    ctx.insert("categories", &state.registry.category_names());

    match state
        .templates
        .render_with_context("preferences.html", &ctx)
    {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html("<h1>Preferences</h1>".to_string())
        }
    }
}

/// Stats page handler
pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let mut ctx = Context::new();
    ctx.insert("instance_name", state.instance_name());
    ctx.insert("engines", &state.registry.names());
    ctx.insert("engine_count", &state.registry.len());

    match state.templates.render_with_context("stats.html", &ctx) {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html("<h1>Stats</h1>".to_string())
        }
    }
}

/// Health check handler
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": crate::VERSION
    }))
}

/// Autocomplete handler
#[derive(Debug, Deserialize)]
pub struct AutocompleteParams {
    pub q: String,
    /// Override autocomplete backend (optional)
    pub backend: Option<String>,
}

pub async fn autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> impl IntoResponse {
    // Get the backend to use (from query param or config)
    let backend_name = params
        .backend
        .as_deref()
        .or_else(|| state.autocomplete_backend())
        .unwrap_or("duckduckgo");

    // Get language from settings
    let lang = &state.settings.ui.default_locale;

    // Fetch suggestions
    let suggestions =
        crate::autocomplete::fetch_suggestions(&state.http_client, backend_name, &params.q, lang)
            .await
            .unwrap_or_default();

    // Return in OpenSearch format: [query, [suggestions...]]
    Json(vec![
        serde_json::Value::String(params.q),
        serde_json::Value::Array(
            suggestions
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
    ])
}

/// Robots.txt handler
pub async fn robots_txt(State(state): State<AppState>) -> impl IntoResponse {
    let content = if state.is_public() {
        "User-agent: *\nAllow: /\nDisallow: /search\nDisallow: /preferences\n"
    } else {
        "User-agent: *\nDisallow: /\n"
    };
    ([(axum::http::header::CONTENT_TYPE, "text/plain")], content)
}

/// Favicon SVG data (Metaseek logo)
const FAVICON_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 92 92"><g transform="translate(-40.921 -17.417)"><circle cx="75.921" cy="53.903" r="30" style="fill:none;stroke:#3050ff;stroke-width:10"/><path d="M67.515 37.915a18 18 0 0 1 21.051 3.313 18 18 0 0 1 3.138 21.078" style="fill:none;stroke:#3050ff;stroke-width:5"/><rect width="18.846" height="39.963" x="3.706" y="122.09" ry="0" style="fill:#3050ff" transform="rotate(-46.235)"/></g></svg>"#;

/// Favicon handler
pub async fn favicon() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "image/svg+xml"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        FAVICON_SVG,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> EngineRegistry {
        let mut registry = EngineRegistry::new();
        registry.register(
            std::sync::Arc::new(crate::engines::bing::Bing::new()),
            Default::default(),
        );
        registry.register(
            std::sync::Arc::new(crate::engines::arxiv::ArXiv::new()),
            Default::default(),
        );
        registry
    }

    fn params(query: &str) -> SearchParams {
        serde_urlencoded::from_str(query).unwrap()
    }

    #[test]
    fn time_range_and_numeric_validation() {
        let registry = registry();
        for (value, expected) in [
            ("day", TimeRange::Day),
            ("week", TimeRange::Week),
            ("month", TimeRange::Month),
            ("year", TimeRange::Year),
        ] {
            let query =
                build_search_query(&params(&format!("q=test&time_range={value}")), &registry)
                    .unwrap();
            assert_eq!(query.time_range, Some(expected));
        }
        for value in ["", "anytime"] {
            assert_eq!(
                build_search_query(
                    &params(&format!("q=test+!week&time_range={value}")),
                    &registry
                )
                .unwrap()
                .time_range,
                None
            );
        }
        assert_eq!(
            build_search_query(&params("q=test+!week"), &registry)
                .unwrap()
                .time_range,
            Some(TimeRange::Week)
        );
        for invalid in [
            "pageno=0",
            "pageno=1001",
            "safesearch=3",
            "time_range=invalid",
        ] {
            assert!(build_search_query(&params(&format!("q=test&{invalid}")), &registry).is_err());
        }
        let query = build_search_query(
            &params("q=test&pageno=2&safesearch=2&language=zh-CN"),
            &registry,
        )
        .unwrap();
        assert_eq!(query.pageno, 2);
        assert_eq!(query.safesearch, 2);
        assert_eq!(query.lang, "zh-CN");
    }

    #[test]
    fn selectors_use_live_categories_and_deduplicate() {
        let registry = registry();
        let query = build_search_query(
            &params("q=test&engines=arxiv,arxiv&categories=general"),
            &registry,
        )
        .unwrap();
        assert_eq!(query.engine_refs, vec![EngineRef::new("arxiv", "science")]);
        let query =
            build_search_query(&params("q=test&categories=general,web"), &registry).unwrap();
        assert_eq!(query.engine_refs.len(), 1);
        let query =
            build_search_query(&params("q=test&engines=,,&categories="), &registry).unwrap();
        assert_eq!(query.engine_refs, vec![EngineRef::new("bing", "general")]);
        let query = build_search_query(&params("q=test+!science"), &registry).unwrap();
        assert_eq!(query.engine_refs, vec![EngineRef::new("arxiv", "science")]);
        let query = build_search_query(&params("q=test+!arxiv&engines=bing"), &registry).unwrap();
        assert_eq!(query.engine_refs, vec![EngineRef::new("bing", "general")]);
        for invalid in ["engines=missing", "categories=missing"] {
            assert!(build_search_query(&params(&format!("q=test&{invalid}")), &registry).is_err());
        }
    }

    #[test]
    fn accepts_only_absolute_web_urls() {
        assert!(http_url("https://example.com/path"));
        for invalid in [
            "/path",
            "javascript:alert(1)",
            "file:///tmp/test",
            "not a url",
        ] {
            assert!(!http_url(invalid));
        }
    }

    #[tokio::test]
    async fn http_invalid_requests_are_json_without_network_calls() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use tower::ServiceExt;
        let state = AppState::new(
            Default::default(),
            registry(),
            crate::network::HttpClient::new().unwrap(),
        )
        .unwrap();
        let router = crate::web::create_router(state);
        for query in [
            "format=json",
            "q=+&format=json",
            "q=test&pageno=0",
            "q=test&pageno=-1",
            "q=test&pageno=abc",
            "q=test&safesearch=256",
            "q=test&safesearch=3",
            "q=test&time_range=nope",
            "q=test&engines=missing",
            "q=test&categories=missing",
            "q=test&format=xml",
            "q=test+!g&format=json",
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/search?{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
            assert!(response.headers()["content-type"]
                .to_str()
                .unwrap()
                .contains("application/json"));
            let body = to_bytes(response.into_body(), 65536).await.unwrap();
            assert!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["message"]
                    .is_string()
            );
        }
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_redirection());
    }

    struct MockEngine {
        name: &'static str,
        url: String,
        fail: bool,
        empty: bool,
    }

    #[async_trait::async_trait]
    impl crate::engines::Engine for MockEngine {
        fn name(&self) -> &str {
            self.name
        }
        fn request(
            &self,
            params: &crate::engines::RequestParams,
        ) -> anyhow::Result<crate::engines::EngineRequest> {
            let mut request = crate::engines::EngineRequest::get(&self.url);
            request.params.insert(
                "time_range".into(),
                params.time_range.map(|r| r.to_string()).unwrap_or_default(),
            );
            request
                .params
                .insert("pageno".into(), params.pageno.to_string());
            Ok(request)
        }
        fn response(
            &self,
            _: crate::engines::EngineResponse,
        ) -> anyhow::Result<crate::engines::EngineResults> {
            if self.fail {
                anyhow::bail!("simulated failure");
            }
            if self.empty {
                return Ok(crate::engines::EngineResults::new());
            }
            let mut result = crate::results::Result::new(
                "https://example.com/paper".into(),
                "Paper".into(),
                self.name.into(),
            )
            .with_position(1);
            result.metadata.published_date = Some("2026-10-05T12:00:00Z".into());
            result.metadata.img_src = Some("https://example.com/image.png".into());
            let invalid = crate::results::Result::new(
                "javascript:alert(1)".into(),
                "Invalid".into(),
                self.name.into(),
            );
            Ok(crate::engines::EngineResults::with_results(vec![
                result, invalid,
            ]))
        }
    }

    #[tokio::test]
    async fn http_search_contract_with_mock_upstream() {
        use axum::body::{to_bytes, Body};
        use axum::http::Request;
        use tower::ServiceExt;
        use wiremock::{
            matchers::{method, query_param},
            Mock, MockServer, ResponseTemplate,
        };
        let upstream = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("time_range", "week"))
            .and(query_param("pageno", "2"))
            .respond_with(ResponseTemplate::new(200))
            .expect(5)
            .mount(&upstream)
            .await;
        for (fail, empty, partial, expected) in [
            (false, false, false, StatusCode::OK),
            (false, true, false, StatusCode::OK),
            (true, false, false, StatusCode::BAD_GATEWAY),
            (true, false, true, StatusCode::OK),
        ] {
            let mut registry = EngineRegistry::new();
            registry.register(
                std::sync::Arc::new(MockEngine {
                    name: "mock",
                    url: upstream.uri(),
                    fail,
                    empty,
                }),
                Default::default(),
            );
            if partial {
                registry.register(
                    std::sync::Arc::new(MockEngine {
                        name: "working",
                        url: upstream.uri(),
                        fail: false,
                        empty: false,
                    }),
                    Default::default(),
                );
            }
            let state = AppState::new(
                Default::default(),
                registry,
                crate::network::HttpClient::new().unwrap(),
            )
            .unwrap();
            let response = crate::web::create_router(state)
                .oneshot(
                    Request::builder()
                        .uri("/search?q=test&format=json&time_range=week&pageno=2")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            let body = to_bytes(response.into_body(), 65536).await.unwrap();
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
            if empty || (fail && !partial) {
                assert_eq!(value["results"].as_array().unwrap().len(), 0);
            } else {
                assert_eq!(value["number_of_results"], 1);
                let result = &value["results"][0];
                assert_eq!(result["content"], "");
                assert_eq!(result["category"], "general");
                assert_eq!(result["publishedDate"], "2026-10-05T12:00:00Z");
                assert_eq!(result["published_date"], result["publishedDate"]);
                assert_eq!(result["thumbnail"], "https://example.com/image.png");
                assert!(result["engines"].is_array());
                assert!(result["score"].is_number());
            }
            if fail {
                assert_eq!(value["engine_errors"].as_array().unwrap().len(), 1);
            }
        }
        upstream.verify().await;
    }

    #[test]
    fn test_parsed_url_array_serialization() {
        let parsed_url = ParsedUrlArray(vec![
            "https".to_string(),
            "example.com".to_string(),
            "/path".to_string(),
            "query=1".to_string(),
            "fragment".to_string(),
            "".to_string(),
        ]);

        let serialized = serde_json::to_string(&parsed_url).unwrap();
        assert_eq!(
            serialized,
            r#"["https","example.com","/path","query=1","fragment",""]"#
        );
    }

    #[test]
    fn test_result_response_has_all_required_fields() {
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
            published_date: Some("2026-10-05T12:00:00Z".into()),
            published_date_compat: Some("2026-10-05T12:00:00Z".into()),
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

        assert_eq!(serialized["publishedDate"], serialized["published_date"]);
        assert_eq!(serialized["publishedDate"], "2026-10-05T12:00:00Z");
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

        let parsed_url = serialized.get("parsed_url").unwrap();
        assert!(parsed_url.is_array());
        assert_eq!(parsed_url.as_array().unwrap().len(), 6);
    }

    #[test]
    fn test_search_response_structure() {
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
}

/// MCP POST handler - receives JSON-RPC requests
pub async fn mcp_post(
    State(_state): State<AppState>,
    Json(request): Json<McpRequest>,
) -> impl IntoResponse {
    // Note: For full MCP support, we'd need to pass AppState to handle requests
    // This is a placeholder that returns an error for now
    // The actual implementation would be in the stdio mode or via a different route
    tracing::warn!("MCP HTTP request received but not fully implemented yet");

    let response = McpResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: None,
        error: Some(McpError::new(
            -32000,
            "MCP HTTP endpoint is experimental. Use stdio mode (--mcp) for full support."
                .to_string(),
        )),
    };

    Json(response)
}
