//! HTTP request handlers

use super::state::AppState;
use crate::query::ParsedQuery;
use crate::search::{EngineRef, SearchQuery};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tera::Context;

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
    #[allow(dead_code)]
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
    pub answers: Vec<String>,
    pub suggestions: Vec<String>,
    pub infoboxes: Vec<serde_json::Value>,
    pub unresponsive_engines: Vec<String>,
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

#[derive(Debug, Serialize)]
pub struct ResultResponse {
    pub url: String,
    pub title: String,
    pub content: Option<String>,
    pub engine: String,
    pub engines: Vec<String>,
    pub score: f64,
    pub category: Option<String>,
    pub thumbnail: Option<String>,
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
    if let Some(url) = state.branding_logo() {
        ctx.insert("logo_url", url);
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

/// Search handler
pub async fn search(State(state): State<AppState>, Query(params): Query<SearchParams>) -> Response {
    // Check for query
    let raw_query = match params.q {
        Some(q) if !q.trim().is_empty() => q,
        _ => return Redirect::to("/").into_response(),
    };

    // Parse query
    let parsed = ParsedQuery::parse(&raw_query);

    // Build engine refs from categories or engines
    let engine_refs = if let Some(ref engines) = params.engines {
        engines
            .split(',')
            .map(|e| EngineRef::new(e.trim(), "general"))
            .collect()
    } else {
        let categories = params
            .categories
            .as_deref()
            .unwrap_or("general")
            .split(',')
            .map(|c| c.trim())
            .collect::<Vec<_>>();

        categories
            .iter()
            .flat_map(|cat| {
                state
                    .registry
                    .get_by_category(cat)
                    .into_iter()
                    .map(|e| EngineRef::new(e.name(), *cat))
            })
            .collect()
    };

    // Build search query
    let mut search_query = SearchQuery::from_parsed(parsed.clone(), engine_refs);
    search_query.pageno = params.pageno.unwrap_or(1);

    if let Some(ref lang) = params.language {
        search_query.lang = lang.clone();
    }

    if let Some(safesearch) = params.safesearch {
        search_query.safesearch = safesearch;
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
            let ordered = results.get_ordered_results();

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
                    .map(|r| ResultResponse {
                        url: r.url,
                        title: r.title,
                        content: r.content,
                        engine: r.engine,
                        engines: r.engines.into_iter().collect(),
                        score: r.score,
                        category: r.category,
                        thumbnail: r.metadata.thumbnail,
                    })
                    .collect(),
                answers: results
                    .get_answers()
                    .into_iter()
                    .map(|a| a.answer)
                    .collect(),
                suggestions: results
                    .get_suggestions()
                    .into_iter()
                    .map(|s| s.text)
                    .collect(),
                infoboxes: vec![],
                unresponsive_engines: results
                    .get_unresponsive()
                    .into_iter()
                    .map(|e| e.name)
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
            if let Some(url) = state.branding_logo() {
                ctx.insert("logo_url", url);
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
            Html("<h1>About</h1><p>SearXNG-RS</p>".to_string())
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

/// Favicon SVG data (SearXNG logo)
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
