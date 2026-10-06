//! Metaseek: A privacy-respecting metasearch engine written in Rust
//!
//! This is the main entry point for the application.

use anyhow::Result;
use metaseek::{
    config::Settings,
    engines::EngineLoader,
    mcp,
    network::HttpClient,
    query::ParsedQuery,
    search::{EngineRef, SearchQuery},
    web::{create_router, AppState},
};
use std::env;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn print_help() {
    println!(
        r#"
Metaseek v{}
A privacy-respecting metasearch engine written in Rust

USAGE:
    metaseek [OPTIONS]

OPTIONS:
    -c, --config <FILE>    Path to configuration file (auto-discover metaseek.yml)
    -p, --port <PORT>      Override server port (default: 8888)
    -b, --bind <ADDR>      Override bind address (default: 127.0.0.1)
    -q, --query <TEXT>     Run a one-off search and print JSON results to stdout
    --mcp                  Start MCP server in stdio mode for AI agent integration
    -h, --help             Print this help message and exit
    -V, --version          Print version information and exit

ENVIRONMENT VARIABLES:
    METASEEK_SETTINGS_PATH  Path to metaseek.yml
    METASEEK_DEBUG          Enable debug mode (true/false)
    METASEEK_PORT           Server port
    METASEEK_BIND_ADDRESS   Bind address
    METASEEK_SECRET_KEY     Secret key for sessions
    ALL_PROXY              Proxy for Tor/HTTP traffic (e.g., socks5h://127.0.0.1:9050)

EXAMPLES:
    # Start with default settings
    metaseek

    # Start with a custom config file
    metaseek --config /etc/metaseek/metaseek.yml

    # Start on a different port and bind to all interfaces
    metaseek --port 9000 --bind 0.0.0.0

    # Start with Tor proxy enabled
    ALL_PROXY=socks5h://127.0.0.1:9050 metaseek

    # Run a one-off search
    metaseek --query "rust programming"

    # Start MCP server in stdio mode
    metaseek --mcp
"#,
        metaseek::VERSION
    );
}

fn print_version() {
    println!("metaseek {}", metaseek::VERSION);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        process::exit(0);
    }

    // Check for version flag
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        print_version();
        process::exit(0);
    }

    // Parse custom arguments and set environment variables
    let mut i = 1;
    let mut query_text: Option<String> = None;
    let mut mcp_mode: bool = false;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    env::set_var("METASEEK_SETTINGS_PATH", &args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --config requires a path argument.");
                    process::exit(1);
                }
            }
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    if let Ok(_p) = args[i + 1].parse::<u16>() {
                        env::set_var("METASEEK_SERVER__PORT", args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Error: --port requires a valid number.");
                        process::exit(1);
                    }
                } else {
                    eprintln!("Error: --port requires a number argument.");
                    process::exit(1);
                }
            }
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    env::set_var("METASEEK_SERVER__BIND_ADDRESS", &args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --bind requires an address argument.");
                    process::exit(1);
                }
            }
            "--query" | "-q" => {
                if i + 1 < args.len() {
                    query_text = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --query requires a text argument.");
                    process::exit(1);
                }
            }
            "--mcp" => {
                mcp_mode = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Initialize logging: suppress all output if --query is used
    let log_level = if query_text.is_some() {
        Level::ERROR // Only show errors, suppress info/warn
    } else {
        Level::INFO
    };

    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    if query_text.is_none() && !mcp_mode {
        info!("Starting Metaseek v{}", metaseek::VERSION);
    }

    // Load configuration
    let settings = Settings::load()?;
    if query_text.is_none() && !mcp_mode {
        info!(
            "Loaded configuration for instance: {}",
            settings.general.instance_name
        );
    }

    // Initialize HTTP client
    let client = HttpClient::with_settings(&settings.outgoing)?;
    if query_text.is_none() && !mcp_mode {
        info!("HTTP client initialized");
    }

    // Load engines
    let registry = EngineLoader::load(&settings)?;
    if query_text.is_none() && !mcp_mode {
        info!("Loaded {} search engines", registry.len());
    }

    // Create application state
    let state = AppState::new(settings.clone(), registry, client)?;
    if query_text.is_none() && !mcp_mode {
        info!("Application state initialized");
    }

    // If --mcp was provided, start MCP server in stdio mode
    if mcp_mode {
        info!("Starting MCP server in stdio mode");
        mcp::run_stdio_server(Arc::new(state)).await?;
        return Ok(());
    }

    // If --query was provided, run a one-off search and exit
    if let Some(query) = query_text {
        run_cli_search(&state, &query).await?;
        return Ok(());
    }

    // Create router
    let app = create_router(state);

    // CLI overrides are already merged into the typed settings.
    let addr = SocketAddr::new(settings.server.bind_address.parse()?, settings.server.port);

    info!("Starting server on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Run a one-off search from the CLI and print JSON results
async fn run_cli_search(state: &AppState, query_text: &str) -> Result<()> {
    // Parse the query
    let parsed = ParsedQuery::parse(query_text);

    // Build search query with default engine refs (all enabled engines)
    let engine_refs: Vec<EngineRef> = state
        .registry
        .names()
        .iter()
        .map(|name| EngineRef::new(name.as_str(), "general"))
        .collect();

    let mut search_query = SearchQuery::from_parsed(parsed.clone(), engine_refs);
    search_query.pageno = 1;
    search_query.lang = "auto".to_string();
    search_query.safesearch = state.settings.search.safe_search;

    // Execute search
    let results = state.search.execute(&search_query).await;

    // Build result objects first to avoid type inference issues
    let ordered_results: Vec<serde_json::Value> = results
        .get_ordered_results()
        .iter()
        .map(|r| {
            serde_json::json!({
                "url": r.url,
                "title": r.title,
                "content": r.content,
                "engine": r.engine,
                "engines": r.engines.iter().cloned().collect::<Vec<_>>(),
                "positions": r.positions,
                "score": r.score,
                "category": r.category,
                "metadata": serde_json::to_value(&r.metadata).unwrap_or(serde_json::Value::Null),
                "result_type": match r.result_type {
                    metaseek::results::ResultType::Default => "default",
                    metaseek::results::ResultType::Image => "image",
                    metaseek::results::ResultType::Video => "video",
                    metaseek::results::ResultType::Map => "map",
                    metaseek::results::ResultType::News => "news",
                    metaseek::results::ResultType::Paper => "paper",
                    metaseek::results::ResultType::File => "file",
                    metaseek::results::ResultType::Code => "code",
                    metaseek::results::ResultType::Answer => "answer",
                    metaseek::results::ResultType::InfoBox => "infobox",
                    metaseek::results::ResultType::Security => "security",
                    metaseek::results::ResultType::Corporate => "corporate",
                },
            })
        })
        .collect();

    let answers: Vec<serde_json::Value> = results
        .get_answers()
        .iter()
        .map(|a| {
            serde_json::json!({
                "answer": a.answer,
                "engine": a.engine,
                "url": a.url,
            })
        })
        .collect();

    let suggestions: Vec<serde_json::Value> = results
        .get_suggestions()
        .iter()
        .map(|s| serde_json::json!({ "text": s.text }))
        .collect();

    let infoboxes: Vec<serde_json::Value> = results
        .get_infoboxes()
        .iter()
        .map(|i| {
            serde_json::json!({
                "title": i.title,
                "content": i.content,
                "engine": i.engine,
                "urls": i.urls,
                "img_src": i.img_src,
            })
        })
        .collect();

    let unresponsive: Vec<serde_json::Value> = results
        .get_unresponsive()
        .iter()
        .map(|e| {
            serde_json::json!({
                "engine": e.name,
                "error": e.error.to_string(),
            })
        })
        .collect();

    let engine_errors: Vec<serde_json::Value> = results
        .get_unresponsive()
        .iter()
        .map(|e| {
            serde_json::json!({
                "engine": e.name,
                "error": e.error.to_string(),
                "error_type": "other",
            })
        })
        .collect();

    // Format results as JSON
    let json_output: serde_json::Value = serde_json::json!({
        "query": query_text.to_string(),
        "number_of_results": results.result_count(),
        "results": ordered_results,
        "answers": answers,
        "suggestions": suggestions,
        "corrections": Vec::<serde_json::Value>::new(),
        "infoboxes": infoboxes,
        "unresponsive_engines": unresponsive,
        "engine_errors": engine_errors,
    });

    // Print JSON to stdout
    println!("{}", serde_json::to_string_pretty(&json_output)?);

    Ok(())
}
