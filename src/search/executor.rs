//! Search execution and orchestration

use super::models::{EngineRef, SearchQuery};
use crate::engines::{Engine, EngineRegistry, RequestParams};
use crate::network::HttpClient;
use crate::results::{EngineError, ResultContainer, Timing};
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Search executor that coordinates searching across multiple engines
pub struct Search {
    /// HTTP client for making requests
    client: HttpClient,
    /// Engine registry
    registry: Arc<EngineRegistry>,
    /// Default timeout
    default_timeout: Duration,
    /// Maximum timeout
    max_timeout: Duration,
}

impl Search {
    /// Create a new search executor
    pub fn new(client: HttpClient, registry: Arc<EngineRegistry>) -> Self {
        Self {
            client,
            registry,
            default_timeout: Duration::from_secs(5),
            max_timeout: Duration::from_secs(30),
        }
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set maximum timeout
    pub fn with_max_timeout(mut self, timeout: Duration) -> Self {
        self.max_timeout = timeout;
        self
    }

    /// Execute a search query across all specified engines
    pub async fn execute(&self, query: &SearchQuery) -> ResultContainer {
        // Get engine weights for scoring
        let weights: HashMap<String, f64> = query
            .engine_refs
            .iter()
            .map(|e| (e.name.clone(), self.registry.get_weight(&e.name)))
            .collect();

        let container = ResultContainer::with_weights(weights);

        // Check for external bang redirect
        if let Some(ref bang) = query.external_bang {
            if let Some(redirect_url) = self.get_external_bang_url(bang, &query.query) {
                container.set_redirect(redirect_url);
                return container;
            }
        }

        // Check for empty query
        if query.is_empty() {
            return container;
        }

        // Execute search on all engines concurrently
        let futures: Vec<_> = query
            .engine_refs
            .iter()
            .filter_map(|engine_ref| {
                let engine = self.registry.get(&engine_ref.name)?;
                Some(self.search_engine(
                    engine.clone(),
                    engine_ref.clone(),
                    query,
                    container.clone(),
                ))
            })
            .collect();

        info!(
            "Executing search '{}' on {} engines",
            query.query,
            futures.len()
        );

        // Wait for all engines to complete
        join_all(futures).await;

        container
    }

    /// Search a single engine
    async fn search_engine(
        &self,
        engine: Arc<dyn Engine>,
        engine_ref: EngineRef,
        query: &SearchQuery,
        container: ResultContainer,
    ) {
        let engine_name = engine.name().to_string();
        let start = Instant::now();

        // Calculate timeout for this engine
        let engine_timeout = Duration::from_secs_f64(
            query
                .timeout_limit
                .unwrap_or(
                    self.registry
                        .get_timeout(&engine_name, self.default_timeout.as_secs_f64()),
                )
                .min(self.max_timeout.as_secs_f64()),
        );

        debug!(
            "Searching engine {} with timeout {:?}",
            engine_name, engine_timeout
        );

        // Build request parameters with engine config merged into engine_data
        // User-supplied API keys take priority over system defaults
        let mut engine_data = query.engine_data.clone();

        // Add API key from engine config only if not already provided by user
        if !engine_data.contains_key("api_key") {
            let engine_config = self.registry.get_config(&engine_name);
            if let Some(config) = engine_config {
                if let Some(ref api_key) = config.api_key {
                    if !api_key.is_empty() {
                        engine_data
                            .insert("api_key".to_string(), serde_json::json!(api_key.clone()));
                    }
                }
            }
        }

        let params = RequestParams {
            query: query.query.clone(),
            pageno: query.pageno,
            lang: query.lang.clone(),
            safesearch: query.safesearch,
            time_range: query.time_range,
            category: engine_ref.category.clone(),
            engine_data,
        };

        // Build the request
        let request = match engine.request(&params) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to build request for {}: {}", engine_name, e);
                container.add_unresponsive(engine_name, EngineError::Unknown);
                return;
            }
        };

        // Execute the request with timeout
        let result = timeout(engine_timeout, self.client.execute(request)).await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(response)) => {
                // Parse the response
                match engine.response(response) {
                    Ok(engine_results) => {
                        let result_count = engine_results.results.len();

                        // Add results to container
                        for mut result in engine_results.results {
                            result.category = Some(engine_ref.category.clone());
                            container.add_result(result);
                        }

                        // Add answers
                        for answer in engine_results.answers {
                            container.add_answer(answer);
                        }

                        // Add suggestions
                        for suggestion in engine_results.suggestions {
                            container.add_suggestion(suggestion);
                        }

                        // Add infoboxes
                        for infobox in engine_results.infoboxes {
                            container.add_infobox(infobox);
                        }

                        // Record timing
                        container.add_timing(Timing {
                            engine: engine_name.clone(),
                            time_ms: elapsed.as_millis() as u64,
                            result_count,
                        });

                        debug!(
                            "Engine {} returned {} results in {:?}",
                            engine_name, result_count, elapsed
                        );
                    }
                    Err(e) => {
                        warn!("Failed to parse response from {}: {}", engine_name, e);
                        let error = if e.to_string().contains("CAPTCHA") {
                            EngineError::Captcha
                        } else if e.to_string().contains("API key")
                            || e.to_string().contains("requires an API key")
                        {
                            EngineError::MissingApiKey
                        } else {
                            EngineError::ParseError
                        };
                        container.add_unresponsive(engine_name, error);
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("Request failed for {}: {}", engine_name, e);
                let error = if e.to_string().contains("timeout") {
                    EngineError::Timeout
                } else if e.to_string().contains("429") {
                    EngineError::TooManyRequests
                } else if e.to_string().contains("403") {
                    EngineError::AccessDenied
                } else {
                    EngineError::NetworkError
                };
                container.add_unresponsive(engine_name, error);
            }
            Err(_) => {
                warn!("Timeout for engine {}", engine_name);
                container.add_unresponsive(engine_name, EngineError::Timeout);
            }
        }
    }

    /// Get redirect URL for external bang
    fn get_external_bang_url(&self, bang: &str, query: &str) -> Option<String> {
        let encoded_query = urlencoding::encode(query);
        match bang {
            "g" => Some(format!("https://www.google.com/search?q={}", encoded_query)),
            "yt" => Some(format!(
                "https://www.youtube.com/results?search_query={}",
                encoded_query
            )),
            "w" | "wp" => Some(format!(
                "https://en.wikipedia.org/wiki/Special:Search?search={}",
                encoded_query
            )),
            "gh" => Some(format!("https://github.com/search?q={}", encoded_query)),
            "so" => Some(format!(
                "https://stackoverflow.com/search?q={}",
                encoded_query
            )),
            "ddg" => Some(format!("https://duckduckgo.com/?q={}", encoded_query)),
            "amazon" => Some(format!("https://www.amazon.com/s?k={}", encoded_query)),
            "imdb" => Some(format!("https://www.imdb.com/find?q={}", encoded_query)),
            _ => None,
        }
    }

    /// Execute search and return results for a specific category
    pub async fn search_category(&self, query: &str, category: &str, page: u32) -> ResultContainer {
        let engines = self.registry.get_by_category(category);

        let engine_refs: Vec<EngineRef> = engines
            .iter()
            .map(|e| EngineRef::new(e.name(), category))
            .collect();

        let search_query = SearchQuery {
            query: query.to_string(),
            engine_refs,
            lang: "all".to_string(),
            safesearch: 0,
            pageno: page,
            time_range: None,
            timeout_limit: None,
            external_bang: None,
            redirect_to_first: false,
            engine_data: HashMap::new(),
        };

        self.execute(&search_query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_executor_creation() {
        let client = HttpClient::new().unwrap();
        let registry = Arc::new(EngineRegistry::new());
        let search = Search::new(client, registry);

        let query = SearchQuery::simple("test");
        let results = search.execute(&query).await;

        assert_eq!(results.result_count(), 0); // No engines registered
    }
}
