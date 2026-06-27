# SearXNG-RS

A privacy-respecting metasearch engine written in Rust, aggregating results from 50+ search engines including Google, Bing, DuckDuckGo, academic databases, Tor hidden services, and more.

## Features

- **Multi-Engine Aggregation**: Query 50+ search engines simultaneously
- **Privacy-First**: No tracking, no cookies, no IP logging
- **Tor Support**: Integrated Ahmia engine for .onion hidden service search
- **RESTful JSON API**: Machine-readable results for automation and agents
- **Custom Branding**: Environment-variable driven theming with local logo embedding
- **CLI Mode**: One-off search execution without server startup
- **Docker Ready**: Production-grade containerization with systemd examples

## Quick Start

### Prerequisites

- Rust 1.70+ (for compilation)
- Docker (optional, for containerized deployment)
- Tor proxy (optional, for Ahmia engine)

### Installation

```bash
# Clone the repository
git clone https://github.com/searxng-rs/searxng-rs.git
cd searxng-rs

# Build from source
cargo build --release

# Or run with Docker
docker-compose up -d
```

### Configuration

Create `config/settings.yml` from the example:

```bash
cp config/settings.yml.example config/settings.yml
```

Key configuration sections:

- **server**: Bind address, port, secret key
- **branding**: Custom name, logo, tagline, accent color
- **engines**: Enable/disable individual search engines
- **search**: Default categories, time ranges, safesearch

### Environment Variables

Override settings via environment variables:

```bash
# Branding
export SEARXNG_BRANDING_NAME="My Search Engine"
export SEARXNG_BRANDING_LOGO="/path/to/logo.svg"
export SEARXNG_BRANDING_TAGLINE="Custom tagline"
export SEARXNG_BRANDING_ACCENT_COLOR="#ff6600"

# Server
export SEARXNG_PORT=9000
export SEARXNG_BIND_ADDRESS=0.0.0.0
export SEARXNG_SECRET_KEY="generate-a-secure-random-key"

# Tor Proxy (required for Ahmia engine)
export ALL_PROXY=socks5h://127.0.0.1:9050
```

## Usage

### Web Interface

Start the server:

```bash
./target/release/searxng-rs
```

Navigate to `http://localhost:8888` in your browser.

### CLI Mode (One-Off Search)

Execute a search without starting the server:

```bash
./target/release/searxng-rs --query "rust programming"
```

Output is JSON to stdout, suitable for scripting:

```bash
./target/release/searxng-rs --query "climate change" | jq '.results[] | {title, url}'
```

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

### Query Syntax

Advanced search operators:

| Operator | Description | Example |
|----------|-------------|---------|
| `!engine` | Search specific engine | `!github rust crates` |
| `:lang` | Filter by language | `python tutorial :en` |
| `<N` | Custom timeout (seconds) | `news <3` |
| `!!` | Direct to first result | `!! weather` |
| `!safesearch` | Enable strict filtering | `images !safesearch` |
| `!day/!week/!month/!year` | Time range filter | `tech news !week` |

### Tor Hidden Service Search (Ahmia)

The Ahmia engine requires Tor proxy configuration:

```bash
# Start Tor service
sudo systemctl start tor

# Set proxy environment variable
export ALL_PROXY=socks5h://127.0.0.1:9050

# Run search with Ahmia enabled
./target/release/searxng-rs --query "onion services"
```

Ahmia automatically:
- Connects to the .onion endpoint
- Extracts anti-bot tokens from the homepage
- Caches tokens for subsequent requests
- Parses HTML results into structured data

### Reuters API Integration

Reuters engine uses the official JSON API:

```bash
./target/release/searxng-rs --query "market news" --categories news,financial
```

Results include:
- Article descriptions and kicker categories
- Publication timestamps
- Source attribution
- Multi-engine cross-referencing

## Branding System

### Local Logo Embedding

Specify a local file path for the logo:

```yaml
branding:
  logo: "/assets/logo.svg"
```

The system:
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

### Environment Variable Overrides

Priority order: Environment > Configuration file > Defaults

```bash
# Override all branding elements at runtime
SEARXNG_BRANDING_NAME="Enterprise Search" \
SEARXNG_BRANDING_LOGO="/opt/logos/enterprise.png" \
SEARXNG_BRANDING_TAGLINE="Secure internal search" \
SEARXNG_BRANDING_ACCENT_COLOR="#0066cc" \
./target/release/searxng-rs
```

## Architecture

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

## Deployment

### Docker

```bash
docker-compose up -d
```

Environment variables in `searxng-rs.env`:

```env
SEARXNG_BRANDING_NAME=My Search
SEARXNG_BRANDING_LOGO=/assets/logo.svg
SEARXNG_PORT=8888
SEARXNG_BIND_ADDRESS=0.0.0.0
ALL_PROXY=socks5h://172.17.0.1:9050
```

### Systemd Service

```ini
# /etc/systemd/system/searxng-rs.service
[Unit]
Description=SearXNG-RS Metasearch Engine
After=network.target

[Service]
Type=simple
User=searxng
WorkingDirectory=/opt/searxng-rs
EnvironmentFile=/etc/searxng/searxng-rs.env
ExecStart=/opt/searxng-rs/target/release/searxng-rs
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable searxng-rs
sudo systemctl start searxng-rs
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name search.example.com;

    location / {
        proxy_pass http://localhost:8888;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## API Reference

### Engine Trait

All engines implement the `Engine` trait:

```rust
pub trait Engine {
    fn name(&self) -> &str;
    fn about(&self) -> EngineAbout;
    fn categories(&self) -> Vec<&str>;
    fn supports_paging(&self) -> bool;
    fn request(&self, params: &RequestParams) -> AnyhowResult<EngineRequest>;
    fn response(&self, response: EngineResponse) -> AnyhowResult<EngineResults>;
    
    // Optional
    fn init(&self) -> AnyhowResult<()>;
    fn validate(&self, config: &EngineConfig) -> AnyhowResult<()>;
    fn supports_time_range(&self) -> bool;
    fn supports_safesearch(&self) -> bool;
    fn timeout(&self) -> f64;
    fn weight(&self) -> f64;
    fn results_per_page(&self) -> usize;
}
```

### Result Metadata

```rust
pub struct Result {
    pub url: String,
    pub title: String,
    pub content: Option<String>,
    pub engine: String,
    pub engines: Vec<String>,
    pub positions: Vec<usize>,
    pub score: f64,
    pub category: Option<String>,
    pub result_type: ResultType,
    pub metadata: ResultMetadata,
}

pub struct ResultMetadata {
    pub author: Option<String>,
    pub published_date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub source_code: Option<String>,
    pub thumbnail: Option<String>,
    pub views: Option<u64>,
    pub stars: Option<u64>,
    pub license: Option<String>,
    pub version: Option<String>,
    pub is_official: bool,
}
```

## Testing

```bash
# Run all tests
cargo test

# Run specific engine tests
cargo test --lib engines::ahmia::tests

# Run with verbose output
cargo test -- --nocapture

# Check for compilation errors
cargo check

# Format code
cargo fmt

# Lint for issues
cargo clippy
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Adding New Engines

See `AGENTS.md` for detailed implementation guidelines.

## License

MIT License - see [LICENSE](LICENSE) file for details

## Acknowledgments

- [SearXNG](https://github.com/searxng/searxng) - Original Python implementation
- [Rust](https://www.rust-lang.org) - Systems programming language
- [Tor Project](https://torproject.org) - Anonymous communication network
- All contributors and maintainers

## Support

- **Issues**: [GitHub Issues](https://github.com/searxng-rs/searxng-rs/issues)
- **Documentation**: [Wiki](https://github.com/searxng-rs/searxng-rs/wiki)
- **Community**: [Discord](https://discord.gg/searxng-rs)

---

*Built with privacy and performance in mind.*