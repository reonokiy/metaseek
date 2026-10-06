# Metaseek

Fork of [SearXNG-RS](https://github.com/sempervictus/searxng-rs), retaining its AGPL-3.0 license and upstream attribution. Existing `SEARXNG_*` environment variables remain supported.

[![CI](https://github.com/reonokiy/metaseek/actions/workflows/ci.yml/badge.svg)](https://github.com/reonokiy/metaseek/actions/workflows/ci.yml)

A privacy-respecting metasearch engine written in Rust. This project is a Rust-based implementation inspired by [SearXNG](https://github.com/searxng/searxng).

**Version:** 0.3.0  
**Total Engines:** 51 (45 + 6 new)  
**Tests Passing:** 134 (100% pass rate)  
**Compilation:** 0 errors  
**Build Status:**  Production-ready  
**API Key System:**  Complete validation infrastructure  
**Tor Support:**  Ahmia engine with token caching  
**CLI Mode:**  One-off search without server startup  
**Branding:**  Environment-variable driven theming

---

## What's New in v0.3.0

**Ahmia Tor Engine Hardening:**
- Migrated to .onion hidden service endpoint
- Token-based anti-bot bypass with DashMap caching
- Automatic SOCKS5 proxy configuration via environment variables
- Token expiration tracking for fresh credentials

**Reuters API Migration:**
- Switched from HTML scraping to official JSON API
- Reduced bandwidth and improved reliability
- Extracts article metadata including kicker categories

**Branding System:**
- Environment variable overrides (`SEARXNG_BRANDING_*`)
- Local logo detection and base64 data URI embedding
- MIME type auto-detection (SVG, PNG, JPG, GIF)
- Offline-capable deployments with embedded assets

**CLI Enhancements:**
- `--query` mode for headless search automation
- Suppressed logging in query mode
- JSON output to stdout for CI/CD integration
- Full argument parsing (`--config`, `--port`, `--bind`)

**New Usage Documentation:**
- Dedicated `/usage` page with query syntax guide
- API endpoint documentation for agents
- Tor proxy configuration examples

---

## Quick Start

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/reonokiy/metaseek.git
cd metaseek
cargo build --release

# Run server
./target/release/metaseek
# Server starts at http://127.0.0.1:8888

# One-off search (CLI mode)
./target/release/metaseek --query "rust programming"
```

## Features

- **51 Search Engines** - Google, Bing, DuckDuckGo, GitHub, arXiv, Semantic Scholar, and more
- **Privacy-First** - No user tracking, tracker URL removal
- **Multiple Formats** - HTML, JSON, CSV responses
- **API Key Support** - LinkedIn, GitHub, GitLab with clear error messages
- **Built-in Plugins** - Calculator, unit converter, hash generator, tracker remover
- **Autocomplete** - DuckDuckGo, Google, Wikipedia, Brave backends
- **Query Syntax** - Language filters, engine bangs, time ranges
- **Tor Hidden Service Search** - Ahmia engine with proxy support
- **Custom Branding** - Environment-driven theming with local logo embedding

## Supported Search Engines

**51 engines across 7 categories:**

| Category | Engines |
|----------|---------|
| General | Google, Bing, DuckDuckGo, Brave, Wikipedia |
| Academic | arXiv, Semantic Scholar, OpenAlex, Crossref, Google Scholar, PubMed, PDBe, BASE, SCANR Structures |
| Code | GitHub, GitLab, Stack Overflow, Crates.io, PyPI, npm, hex.pm, Docker Hub, and more |
| OS Packages | Arch Linux, Alpine Linux, Void Linux, CachyOS, Repology |
| Corporate | OpenCorporates, SEC EDGAR, Crunchbase, IMDb |
| Security | NVD, Ahmia |
| News | Reuters, Bloomberg, AP News |

**API Key Engines:** LinkedIn Companies (required), GitHub/GitLab (optional)

## Installation

### Prerequisites

- Rust 1.95 (with Cargo; tested toolchain)

### Build

```bash
# Debug build
cargo build

# Release build (recommended)
cargo build --release
```

### Run

```bash
# Using cargo
cargo run --release

# Or run the binary directly
./target/release/metaseek
```

The server starts at `http://127.0.0.1:8888` by default.

### Docker

```bash
docker build -t metaseek .
docker run --rm -p 127.0.0.1:8888:8888 metaseek --bind 0.0.0.0 --port 8888
```

Build the fork locally; no Metaseek container images have been published yet.

### Systemd Service

Create `/etc/systemd/system/metaseek.service`:

```ini
[Unit]
Description=Metaseek Metasearch Engine
After=network.target

[Service]
Type=simple
User=searxng
WorkingDirectory=/opt/metaseek
EnvironmentFile=/etc/searxng/metaseek.env
ExecStart=/opt/metaseek/target/release/metaseek
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable metaseek
sudo systemctl start metaseek
```

## Configuration

The application looks for `settings.yml` in these locations (in order):

1. `$SEARXNG_SETTINGS_PATH` environment variable
2. `./settings.yml`
3. `./config/settings.yml`
4. `/etc/searxng/settings.yml`
5. `~/.config/metaseek/settings.yml`

### Example Configuration

```yaml
general:
  debug: false
  instance_name: "Metaseek"
  enable_metrics: true

search:
  safe_search: 0              # 0=None, 1=Moderate, 2=Strict
  autocomplete: "duckduckgo"
  default_lang: "auto"

server:
  port: 8888
  bind_address: "127.0.0.1"
  secret_key: "change-me-in-production"

branding:
  name: "My Search Engine"
  logo: "/assets/logo.svg"    # Local path or URL
  tagline: "Privacy-first search"
  accent_color: "#e94560"

engines:
  # Free engines (no API key required)
  - name: google
    disabled: false
  - name: duckduckgo
    disabled: false
  
  # Optional API key engines (work without key, better with)
  - name: github
    disabled: false
    # api_key: "your_github_token"  # Optional: Higher rate limits
  
  # Required API key engines (must configure)
  - name: linkedin_companies
    disabled: true  # Enabled only with API key
    api_key: "your_linkedin_api_key"
```

### Branding System

**Environment Variable Overrides:**

```bash
# Override branding at runtime
export SEARXNG_BRANDING_NAME="Enterprise Search"
export SEARXNG_BRANDING_LOGO="/opt/logos/enterprise.png"
export SEARXNG_BRANDING_TAGLINE="Secure internal search"
export SEARXNG_BRANDING_ACCENT_COLOR="#0066cc"
```

**Local Logo Embedding:**

When a logo path begins with `/` or `./`, the system:
1. Detects file extension (`.svg`, `.png`, `.jpg`, `.gif`)
2. Reads file contents
3. Encodes as base64
4. Constructs data URI: `data:image/svg+xml;base64,<encoded-data>`
5. Embeds directly in HTML responses

Benefits:
- No external HTTP requests
- Offline-capable deployments
- Faster page loads
- Complete privacy (no third-party tracking)

### Tor Proxy Configuration (Ahmia Engine)

For Ahmia Tor hidden service search, configure proxy:

```bash
# Start Tor service
sudo systemctl start tor

# Set proxy environment variable
export ALL_PROXY=socks5h://127.0.0.1:9050

# Or in settings.yml
env:
  ALL_PROXY: "socks5h://127.0.0.1:9050"
```

The Ahmia engine automatically:
- Connects to the .onion endpoint
- Extracts anti-bot tokens from the homepage
- Caches tokens for subsequent requests
- Parses HTML results into structured data

### Engine Configuration Reference

See `config/settings.yml.example` for complete engine configuration.

**API Key Setup:**
- Required keys: LinkedIn Companies (see [API_KEY_SETUP.md](API_KEY_SETUP.md))
- Optional keys: GitHub, GitLab (higher rate limits)
- All other engines: Free, no keys needed

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SEARXNG_SETTINGS_PATH` | Path to settings.yml | - |
| `SEARXNG_DEBUG` | Enable debug mode | `false` |
| `SEARXNG_PORT` | Server port | `8888` |
| `SEARXNG_BIND_ADDRESS` | Bind address | `127.0.0.1` |
| `SEARXNG_SECRET_KEY` | Secret key for sessions | - |
| `ALL_PROXY` | Proxy for Tor/HTTP traffic | - |
| `HTTP_PROXY` | HTTP proxy fallback | - |
| `HTTPS_PROXY` | HTTPS proxy fallback | - |
| `SOCKS_PROXY` | SOCKS proxy fallback | - |

## Query Syntax

| Syntax | Example | Description |
|--------|---------|-------------|
| `:lang` | `rust :en` | Filter by language |
| `!engine` | `rust !github` | Search specific engine |
| `!category` | `cats !images` | Search category |
| `<timeout` | `query <10` | Custom timeout (seconds) |
| `!safesearch` | `query !safesearch` | Enable safe search |
| `!nosafesearch` | `query !nosafesearch` | Disable safe search |
| `!day/!week/!month/!year` | `news !week` | Time range filter |
| `!!` | `!! query` | Redirect to first result |

## CLI Mode (One-Off Search)

Execute a search without starting the server:

```bash
# Basic query
./target/release/metaseek --query "rust programming"

# With custom config
./target/release/metaseek --config /etc/searxng/settings.yml --query "climate change"

# Pipe to jq for JSON processing
./target/release/metaseek --query "ai news" | jq '.results[] | {title, url}'
```

Output is JSON to stdout, suitable for:
- CI/CD integration
- Monitoring scripts
- Agent-driven search workflows

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Home page |
| `GET /search` | Search results |
| `GET /autocomplete` | Search suggestions |
| `GET /preferences` | User preferences |
| `GET /stats` | Instance statistics |
| `GET /about` | About page |
| `GET /usage` | Query syntax guide |
| `GET /health` | Health check |

### JSON API

Query the REST API directly:

```bash
curl "http://localhost:8888/search?q=artificial+intelligence&format=json"
```

Parameters:
- `q`: Search query (required)
- `format`: Output format (`json`, `html`, `xml`)
- `categories`: Comma-separated category list (`general,news,academic`)
- `engines`: Comma-separated engine list (`google,duckduckgo,wikipedia`)
- `language`: Language code (`en`, `de`, `fr`)
- `safesearch`: `0` (off), `1` (moderate), `2` (strict)
- `time_range`: `day`, `week`, `month`, `year`
- `pageno`: Page number (default: 1)

### Example Response

```json
{
  "query": "rust programming",
  "number_of_results": 15,
  "results": [
    {
      "url": "https://www.rust-lang.org",
      "title": "Rust Programming Language",
      "content": "A language empowering everyone to build reliable...",
      "engine": "google",
      "engines": ["google", "duckduckgo"],
      "positions": [1, 3],
      "score": 2.5,
      "category": "general",
      "parsed_url": ["https", "www.rust-lang.org", "/", "", "", ""]
    }
  ],
  "answers": [],
  "suggestions": ["rust language", "rust tutorial"],
  "corrections": [],
  "infoboxes": [],
  "unresponsive_engines": [],
  "engine_errors": []
}
```

## Project Structure

```
src/
  engines/          # Search engine implementations (50+ modules)
    mod.rs          # Engine declarations
    registry.rs     # Engine registration and loading
    traits.rs       # Engine trait definition
    <engine>.rs     # Individual engine (e.g., ahmia.rs, reuters.rs)
  results/          # Result types and metadata
  query/            # Query parsing and operator handling
  search/           # Parallel engine execution and result merging
  config/           # Settings loading and validation
  web/              # HTTP server and template rendering
    handlers.rs     # Route handlers (search, index, about)
    state.rs        # Application state (branding, config)
    templates.rs    # Tera template engine integration
  network/          # HTTP client with proxy support
```

### Engine Lifecycle

1. **Load**: `EngineLoader` reads `settings.yml`, instantiates enabled engines
2. **Validate**: Each engine checks API key requirements (if applicable)
3. **Request**: `Engine::request()` builds HTTP query with parameters
4. **Execute**: Parallel `reqwest` calls with configurable timeouts
5. **Parse**: `Engine::response()` extracts JSON/HTML/XML into `Result` objects
6. **Merge**: `Search` executor deduplicates, ranks, and aggregates results
7. **Return**: JSON API or HTML template with unified result set

### Token Caching (Ahmia)

```
User Request
    |
    v
Check DashMap Cache (token key: "ahmia_tokens")
    |
    +-- Hit (not expired) --> Use cached token
    |
    +-- Miss/Expired --> Fetch from .onion homepage
                              |
                              v
                         Parse HTML for hidden inputs
                              |
                              v
                         Store (name, value, timestamp) in DashMap
                              |
                              v
                         Inject token into search request
```

## Tech Stack

- **Runtime**: Tokio
- **Web Framework**: Axum
- **HTTP Client**: Reqwest
- **Templates**: Tera
- **Caching**: Moka, DashMap
- **Serialization**: Serde, Serde_json

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check for compilation errors
cargo check

# Format code
cargo fmt

# Lint for issues
cargo clippy
```

## Feature Parity with SearXNG

| Feature | metaseek | SearXNG |
|---------|-----------|---------|
| Total Engines | 51 | 215+ |
| HTML Output |  |  |
| JSON Output |  |  |
| CSV Output |  |  |
| RSS Output |  |  |
| Calculator Plugin |  |  |
| Unit Converter |  |  |
| Hash Generator |  |  |
| Tracker URL Remover |  |  |
| DuckDuckGo Autocomplete |  |  |
| Google Autocomplete |  |  |
| Wikipedia Autocomplete |  |  |
| Language Filter |  |  |
| Engine Bangs |  |  |
| Timeout Control |  |  |
| Safe Search |  |  |
| Time Range Filter |  |  |
| First Result Redirect |  |  |
| No User Tracking |  |  |
| Tracker URL Removal |  |  |
| Image Proxy |  |  |
| No Referrer Headers |  |  |
| Tor Support (Ahmia) |  |  |
| YAML Config |  |  |
| Environment Variables |  |  |
| Per-Engine Settings |  |  |
| Rate Limiting |  |  |
| Health Endpoint |  |  |
| Statistics Endpoint |  |  |
| Engine Metrics |  |  |
| Docker Images |  |  |
| Single Binary |  |  |
| Low Memory Footprint |  |  |

### Current Limitations

- **Missing Categories:** Images, Videos, Maps, Music, Torrents, Books, Translation, Shopping
- **Limited Engines:** 51 vs 215+ in SearXNG
- **No Image Proxy:** Simplified implementation
- **Basic Themes:** Single theme (simple)
- **Limited Localization:** 3 languages (en, de, fr)

### API Key Engines

- **LinkedIn Companies:** Requires API key (disabled by default)
- **GitHub/GitLab:** Optional API keys for higher rate limits
- **All other engines:** Free, no API keys required

## License

AGPL-3.0 - See [LICENSE](LICENSE) for details.

---

## Quick Links

- **[API Key Setup Guide](API_KEY_SETUP.md)** - Configure API keys
- **[Developer Quick Ref](DEVELOPER_QUICK_REF.md)** - Add new engines
- **[Agent Guide](AGENTS.md)** - Development guidelines
- **[Engine Expansion Plan](ENGINE_EXPANSION_PLAN.md)** - Roadmap
- **[Implementation Summary](IMPLEMENTATION_SUMMARY.md)** - Current status

## Fork Information

This repository is a fork maintained under the **Sempervictus** organization.  
Original inspiration: [SearXNG](https://github.com/searxng/searxng)

**Contact:** See repository for maintainers and support information.