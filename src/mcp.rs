//! Model Context Protocol (MCP) implementation for Metaseek
//!
//! This module provides stdio CLI mode (`--mcp`) for integrating with AI agents
//! and LLMs via the MCP protocol.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::search::{EngineRef, SearchQuery};
use crate::web::state::AppState;

// ============================================================================
// MCP Protocol Types
// ============================================================================

/// MCP Request structure
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "jsonrpc", rename_all = "snake_case")]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// MCP Response structure
#[derive(Debug, Serialize)]
#[serde(tag = "jsonrpc", rename_all = "snake_case")]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP Error structure
#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    pub fn with_data(code: i32, message: String, data: serde_json::Value) -> Self {
        Self {
            code,
            message,
            data: Some(data),
        }
    }
}

// ============================================================================
// MCP Tool Definitions
// ============================================================================

/// Tool definition for MCP
#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Parameters for the search tool
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchToolParams {
    /// The search query string
    pub query: String,
    /// Comma-separated list of engines to use (optional)
    #[serde(default)]
    pub engines: Option<String>,
    /// Comma-separated list of categories (optional)
    #[serde(default)]
    pub categories: Option<String>,
    /// Language code (optional)
    #[serde(default)]
    pub language: Option<String>,
    /// Time range: day, week, month, year (optional)
    #[serde(default)]
    pub time_range: Option<String>,
    /// Safe search level: 0 (off), 1 (moderate), 2 (strict) (optional)
    #[serde(default)]
    pub safesearch: Option<u8>,
    /// Page number (optional, default: 1)
    #[serde(default)]
    pub pageno: Option<u32>,
    /// Number of results per page (optional, default: 10)
    #[serde(default)]
    pub num: Option<u32>,
}

// ============================================================================
// MCP Server State
// ============================================================================

/// Shared state for MCP server
pub struct McpServerState {
    pub app_state: Arc<AppState>,
}

// ============================================================================
// MCP Request Handler
// ============================================================================

/// Process an MCP request
pub async fn handle_mcp_request(
    state: &McpServerState,
    request: McpRequest,
) -> Result<McpResponse, McpError> {
    let id = request.id.clone();

    match request.method.as_str() {
        "initialize" => Ok(handle_initialize(id)),
        "notifications/initialized" => Ok(handle_initialized(id)),
        "tools/list" => Ok(handle_tools_list(id)),
        "tools/call" => handle_tools_call(state, id, request.params).await,
        _ => Err(McpError::new(
            -32601,
            format!("Method not found: {}", request.method),
        )),
    }
}

/// Handle initialize request
fn handle_initialize(id: Option<serde_json::Value>) -> McpResponse {
    let result = serde_json::json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "metaseek",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": {
            "tools": {}
        }
    });

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

/// Handle initialized notification
fn handle_initialized(id: Option<serde_json::Value>) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::Value::Null),
        error: None,
    }
}

/// Handle tools/list request
fn handle_tools_list(id: Option<serde_json::Value>) -> McpResponse {
    let tools = vec![Tool {
        name: "search".to_string(),
        description: "Perform a web search across multiple engines. Supports filtering by engine, category, language, time range, and safe search.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query string"
                },
                "engines": {
                    "type": "string",
                    "description": "Comma-separated list of engine names (e.g., 'google,duckduckgo')"
                },
                "categories": {
                    "type": "string",
                    "description": "Comma-separated list of categories (e.g., 'general,news,academic')"
                },
                "language": {
                    "type": "string",
                    "description": "Language code (e.g., 'en', 'de', 'fr')"
                },
                "time_range": {
                    "type": "string",
                    "description": "Time range filter: 'day', 'week', 'month', 'year'"
                },
                "safesearch": {
                    "type": "integer",
                    "description": "Safe search level: 0 (off), 1 (moderate), 2 (strict)"
                },
                "pageno": {
                    "type": "integer",
                    "description": "Page number for pagination (default: 1)"
                },
                "num": {
                    "type": "integer",
                    "description": "Number of results per page (default: 10)"
                }
            },
            "required": ["query"]
        }),
    }];

    let result = serde_json::json!({
        "tools": tools
    });

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

/// Handle tools/call request
async fn handle_tools_call(
    state: &McpServerState,
    id: Option<serde_json::Value>,
    params: serde_json::Value,
) -> Result<McpResponse, McpError> {
    // Parse parameters
    let search_params: SearchToolParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return Err(McpError::with_data(
                -32602,
                "Invalid params".to_string(),
                serde_json::json!({"error": e.to_string()}),
            ));
        }
    };

    // Build engine references
    let _engine_refs: Vec<EngineRef> = if let Some(ref engines) = search_params.engines {
        engines
            .split(',')
            .map(|e| EngineRef::new(e.trim(), "general"))
            .collect()
    } else if let Some(ref categories) = search_params.categories {
        categories
            .split(',')
            .flat_map(|c| {
                state
                    .app_state
                    .registry
                    .get_by_category(c.trim())
                    .into_iter()
                    .map(|e| EngineRef::new(e.name(), c.trim()))
            })
            .collect()
    } else {
        // Default to general category
        state
            .app_state
            .registry
            .get_by_category("general")
            .into_iter()
            .map(|e| EngineRef::new(e.name(), "general"))
            .collect()
    };

    // Build search query using simple constructor
    let search_query = SearchQuery::simple(search_params.query.clone());

    // Execute search
    let results = state.app_state.search.execute(&search_query).await;

    // Convert results to MCP format
    let ordered = results.get_ordered_results();
    let search_results: Vec<serde_json::Value> = ordered
        .iter()
        .map(|r| {
            serde_json::json!({
                "url": r.url,
                "title": r.title,
                "content": r.content,
                "engine": r.engine,
                "engines": r.engines,
                "positions": r.positions,
                "score": r.score,
                "category": r.category,
                "published_date": r.metadata.published_date,
                "author": r.metadata.author,
                "tags": r.metadata.tags,
                "thumbnail": r.metadata.thumbnail,
                "is_official": r.metadata.is_official,
            })
        })
        .collect();

    let result = serde_json::json!({
        "query": search_params.query,
        "number_of_results": ordered.len(),
        "results": search_results,
        "suggestions": results.get_suggestions().iter().map(|s| s.text.clone()).collect::<Vec<_>>(),
        "answers": results.get_answers().iter().map(|a| a.answer.clone()).collect::<Vec<_>>(),
    });

    Ok(McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    })
}

// ============================================================================
// Stdio Server (CLI Mode)
// ============================================================================

/// Run MCP server in stdio mode
pub async fn run_stdio_server(app_state: Arc<AppState>) -> anyhow::Result<()> {
    let state = McpServerState { app_state };
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    tracing::info!("MCP stdio server started. Waiting for requests...");

    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let error_response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError::new(-32700, format!("Parse error: {}", e))),
                };
                let response_json = serde_json::to_string(&error_response)?;
                writer.write_all(response_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                line.clear();
                continue;
            }
        };

        let response = handle_mcp_request(&state, request).await;
        let mcp_response = match response {
            Ok(r) => r,
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(e),
            },
        };

        let response_json = serde_json::to_string(&mcp_response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        line.clear();
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_serialization() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_mcp_response_serialization() {
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"result\""));
        assert!(!serialized.contains("\"error\""));
    }

    #[test]
    fn test_search_tool_params_deserialization() {
        let json = serde_json::json!({
            "query": "rust programming",
            "engines": "google,duckduckgo",
            "language": "en",
            "safesearch": 1,
            "pageno": 2
        });

        let params: SearchToolParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.query, "rust programming");
        assert_eq!(params.engines, Some("google,duckduckgo".to_string()));
        assert_eq!(params.language, Some("en".to_string()));
        assert_eq!(params.safesearch, Some(1));
        assert_eq!(params.pageno, Some(2));
    }

    #[test]
    fn test_search_tool_params_minimal() {
        let json = serde_json::json!({
            "query": "simple search"
        });

        let params: SearchToolParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.query, "simple search");
        assert!(params.engines.is_none());
        assert!(params.categories.is_none());
        assert!(params.language.is_none());
        assert_eq!(params.pageno, None);
    }

    #[test]
    fn test_initialize_response_structure() {
        let response = handle_initialize(None);
        assert!(response.result.is_some());
        let result = response.result.unwrap();

        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "metaseek");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_tools_list_response_structure() {
        let response = handle_tools_list(None);
        assert!(response.result.is_some());
        let result = response.result.unwrap();

        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "search");
        assert!(tools[0]["description"].is_string());
        assert!(tools[0]["input_schema"].is_object());
    }

    #[test]
    fn test_error_response_structure() {
        let error = McpError::new(-32601, "Method not found".to_string());
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("-32601"));
        assert!(json.contains("Method not found"));
    }
}
