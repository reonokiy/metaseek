# SearXNG-RS

[![CI](https://github.com/geoffsee/searxng-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/geoffsee/searxng-rs/actions/workflows/ci.yml)

A privacy-respecting metasearch engine written in Rust. This project is a Rust-based implementation inspired by [SearXNG](https://github.com/searxng/searxng).

**Status:** Early development - core functionality works but expect rough edges.

## Features

- **Multi-Engine Search** - Aggregates results from multiple search engines
- **Privacy-First** - No user tracking or profiling, tracker URL removal
- **Query Syntax** - Language filters, engine bangs, time ranges
- **Built-in Plugins** - Calculator, unit converter, hash generator, tracker remover
- **Autocomplete** - Search suggestions from configurable backends
- **Basic i18n** - UI translations for English, German, and French

## Supported Search Engines

| Engine | Categories |
|--------|------------|
| Google | General |
| Bing | General |
| DuckDuckGo | General |
| Brave | General |
| Wikipedia | General |
| GitHub | IT |
| Stack Overflow | IT |
| YouTube | Videos |
| arXiv | Science |

## Installation

### Prerequisites

- Rust 1.70+ (with Cargo)

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
./target/release/searxng-rs
```

The server starts at `http://127.0.0.1:8888` by default.

### Docker

```bash
docker run -p 8888:8888 ghcr.io/geoffsee/searxng-rs:latest
```

Multi-arch images (amd64/arm64) are available on GHCR.

## Configuration

The application looks for `settings.yml` in these locations (in order):

1. `$SEARXNG_SETTINGS_PATH` environment variable
2. `./settings.yml`
3. `./config/settings.yml`
4. `/etc/searxng/settings.yml`
5. `~/.config/searxng-rs/settings.yml`

### Example Configuration

```yaml
general:
  debug: false
  instance_name: "SearXNG"
  enable_metrics: true

search:
  safe_search: 0              # 0=None, 1=Moderate, 2=Strict
  autocomplete: "duckduckgo"
  default_lang: "auto"

server:
  port: 8888
  bind_address: "127.0.0.1"
  secret_key: "change-me-in-production"

engines:
  - name: google
    disabled: false
  - name: duckduckgo
    disabled: false
  - name: brave
    disabled: false
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SEARXNG_SETTINGS_PATH` | Path to settings.yml | - |
| `SEARXNG_DEBUG` | Enable debug mode | `false` |
| `SEARXNG_PORT` | Server port | `8888` |
| `SEARXNG_BIND_ADDRESS` | Bind address | `127.0.0.1` |
| `SEARXNG_SECRET_KEY` | Secret key for sessions | - |

## Query Syntax

| Syntax | Example | Description |
|--------|---------|-------------|
| `:lang` | `rust :en` | Filter by language |
| `!engine` | `rust !github` | Search specific engine |
| `!category` | `cats !images` | Search category |
| `<timeout` | `query <10` | Custom timeout (seconds) |
| `!safesearch` | `query !safesearch` | Enable safe search |
| `!nosafesearch` | `query !nosafesearch` | Disable safe search |
| `!day/week/month/year` | `news !week` | Time range filter |
| `!!` | `!! query` | Redirect to first result |

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Home page |
| `GET /search` | Search results |
| `GET /autocomplete` | Search suggestions |
| `GET /preferences` | User preferences |
| `GET /stats` | Instance statistics |
| `GET /health` | Health check |

## Project Structure

```
src/
├── engines/        # Search engine implementations
├── web/            # HTTP server and routes
├── search/         # Search orchestration
├── query/          # Query parsing
├── results/        # Result types
├── plugins/        # Built-in plugins
├── autocomplete/   # Autocomplete backends
├── config/         # Configuration
├── network/        # HTTP client
├── cache/          # Caching layer
├── locales/        # Translations
└── templates/      # HTML templates
```

## Tech Stack

- **Runtime**: Tokio
- **Web Framework**: Axum
- **HTTP Client**: Reqwest
- **Templates**: Tera
- **Caching**: Moka

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

## Feature Parity with SearXNG

This table compares the features of searxng-rs with the original [SearXNG](https://github.com/searxng/searxng) Python implementation.

### Search Engines

| Category | searxng-rs | SearXNG |
|----------|-----------|---------|
| Total Engines | 9 | 215+ |
| General Search | Google, Bing, DuckDuckGo, Brave, Wikipedia | Google, Bing, Brave, DuckDuckGo, Yandex, Qwant, Startpage, Yahoo, and many more |
| Images | - | Google Images, Bing Images, Flickr, Unsplash, Pixabay, DeviantArt, and more |
| Videos | YouTube | YouTube, Dailymotion, Vimeo, PeerTube, Invidious, Bilibili, and more |
| News | - | Google News, Bing News, Reuters, Yahoo News, and more |
| Maps | - | OpenStreetMap, Apple Maps |
| IT/Code | GitHub, Stack Overflow | GitHub, GitLab, Gitea, NPM, PyPI, Crates.io, Docker Hub, and more |
| Science/Academic | arXiv | arXiv, Google Scholar, PubMed, Crossref, Semantic Scholar, and more |
| Music | - | Bandcamp, SoundCloud, Spotify, Deezer, and more |
| Torrents/Files | - | Pirate Bay, 1337x, KickAss, and more |
| Books | - | Anna's Archive, Z-Library, OpenLibrary, Goodreads |
| Translation | - | DeepL, LibreTranslate, Lingva |
| Shopping | - | eBay, Amazon integrations |

### Output Formats

| Format | searxng-rs | SearXNG |
|--------|-----------|---------|
| HTML | ✅ | ✅ |
| JSON | ✅ | ✅ |
| CSV | ✅ | ✅ |
| RSS | ✅ | ✅ |

### Plugins

| Plugin | searxng-rs | SearXNG |
|--------|-----------|---------|
| Calculator | ✅ | ✅ |
| Unit Converter | ✅ | ✅ |
| Hash Generator | ✅ | ✅ |
| Tracker URL Remover | ✅ | ✅ |
| Self Info | - | ✅ |
| Ahmia Filter | - | ✅ |
| Hostnames Rewrite | - | ✅ |
| Time Zone | - | ✅ |
| Tor Check | - | ✅ |
| Infinite Scroll | - | ✅ |
| OA DOI Rewrite | - | ✅ |

### Autocomplete Backends

| Backend | searxng-rs | SearXNG |
|---------|-----------|---------|
| DuckDuckGo | ✅ | ✅ |
| Google | ✅ | ✅ |
| Wikipedia | ✅ | ✅ |
| Brave | ✅ | ✅ |
| Qwant | ✅ | ✅ |
| Other backends | - | 10+ more |

### Query Syntax

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Language filter (`:en`) | ✅ | ✅ |
| Engine bangs (`!google`) | ✅ | ✅ |
| Category bangs (`!images`) | ✅ | ✅ |
| External bangs (`!g`, `!yt`) | ✅ | ✅ |
| Timeout control (`<10`) | ✅ | - |
| Safe search toggle | ✅ | ✅ |
| Time range (`!day`, `!week`) | ✅ | ✅ |
| First result redirect (`!!`) | ✅ | ✅ |

### Privacy Features

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| No user tracking | ✅ | ✅ |
| Tracker URL removal | ✅ | ✅ |
| Image proxy | ✅ | ✅ |
| No referrer headers | ✅ | ✅ |
| Tor support | - | ✅ |
| Alternative frontend redirects | - | ✅ |
| POST method option | - | ✅ |

### UI & Themes

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Themes | 1 (default) | Multiple (simple with auto/light/dark/black) |
| Responsive design | ✅ | ✅ |
| Preferences page | ✅ | ✅ |
| Statistics page | ✅ | ✅ |
| Hotkeys | - | ✅ (default + vim mode) |
| Infinite scroll | - | ✅ |

### Localization

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Languages supported | 3 (en, de, fr) | 30+ |
| RTL language support | - | ✅ |
| Browser language detection | ✅ | ✅ |

### Configuration

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| YAML config file | ✅ | ✅ |
| Environment variables | ✅ | ✅ |
| Per-engine settings | ✅ | ✅ |
| Rate limiting | ✅ | ✅ |
| Redis/Valkey support | Partial | ✅ |

### API & Monitoring

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Health endpoint | ✅ | ✅ |
| Statistics endpoint | ✅ | ✅ |
| Engine metrics | ✅ | ✅ |
| OpenMetrics export | - | ✅ |
| Engine checker | - | ✅ |

### Result Types

| Type | searxng-rs | SearXNG |
|------|-----------|---------|
| Default (web) | ✅ | ✅ |
| Images | ✅ | ✅ |
| Videos | ✅ | ✅ |
| News | ✅ | ✅ |
| Maps | ✅ | ✅ |
| Files | ✅ | ✅ |
| Code | ✅ | ✅ |
| Papers | ✅ | ✅ |
| Infoboxes | ✅ | ✅ |
| Answers (instant) | ✅ | ✅ |

### Performance

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Async runtime | ✅ (Tokio) | ✅ (asyncio) |
| Connection pooling | ✅ | ✅ |
| Result caching | ✅ (Moka) | ✅ (Valkey/Redis) |
| Gzip/Brotli compression | ✅ | ✅ |
| HTTP/2 support | ✅ | ✅ |

### Deployment

| Feature | searxng-rs | SearXNG |
|---------|-----------|---------|
| Docker images | ✅ (multi-arch) | ✅ |
| Single binary | ✅ | - (Python) |
| Memory footprint | Low | Higher |

## License

AGPL-3.0 - See [LICENSE](LICENSE) for details.
