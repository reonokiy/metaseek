# Metaseek: Agent & CLI Usage Guide

This document provides compact, actionable instructions for automated agents and scripts to query Metaseek and retrieve structured JSON results.

---

## 1. One-Off Search (CLI Mode)

Run a search without starting a persistent server. Ideal for CI/CD, cron jobs, or single-shot agent tasks.

### Basic Usage
```bash
./target/release/metaseek --query "your search terms"
```

### Output
- **Format**: JSON to `stdout`
- **Logging**: `INFO`/`WARN` suppressed; only `ERROR` shown.
- **Exit Code**: `0` on success, `1` on error.

### Examples

**Simple Search:**
```bash
./target/release/metaseek --query "rust async programming"
```

**With Custom Config:**
```bash
./target/release/metaseek --config /etc/searxng/settings.yml --query "climate data"
```

**Pipe to `jq` for Processing:**
```bash
./target/release/metaseek --query "ai news" | jq '.results[] | {title, url, engine}'
```

**Filter by Category:**
```bash
./target/release/metaseek --query "python libraries" --categories "it,code"
```

**Filter by Engine:**
```bash
./target/release/metaseek --query "react components" --engines "github,npm"
```

### CLI Arguments Reference

| Flag | Description | Example |
|------|-------------|---------|
| `--query`, `-q` | Search query (required for CLI mode) | `--query "rust"` |
| `--config`, `-c` | Path to `settings.yml` | `-c /etc/searxng/settings.yml` |
| `--port`, `-p` | Override server port (ignored in CLI) | `-p 9000` |
| `--bind`, `-b` | Override bind address (ignored in CLI) | `-b 0.0.0.0` |
| `--categories` | Comma-separated category filter | `--categories "general,news"` |
| `--engines` | Comma-separated engine filter | `--engines "google,duckduckgo"` |
| `--language` | Filter by language code | `--language "en"` |
| `--time_range` | Filter by time | `--time_range "week"` |
| `--pageno` | Page number | `--pageno 2` |

---

## 2. HTTP API (Persistent Server)

Start the server once and query it repeatedly via HTTP. Best for high-frequency agent loops.

### Start Server
```bash
./target/release/metaseek
# Runs on http://127.0.0.1:8888 by default
```

### API Endpoint
```
GET /search?q={query}&format=json
```

### Query Parameters

| Param | Type | Description | Default |
|-------|------|-------------|---------|
| `q` | string | Search query | **Required** |
| `format` | string | Output format (`json`, `html`, `csv`) | `json` |
| `categories` | string | Comma-separated categories | All |
| `engines` | string | Comma-separated engine names | All |
| `language` | string | Language code (e.g., `en`, `de`, `fr`) | `auto` |
| `safesearch` | int | `0` (off), `1` (moderate), `2` (strict) | `0` |
| `time_range` | string | `day`, `week`, `month`, `year` | None |
| `pageno` | int | Page number | `1` |
| `num` | int | Results per page | `10` |

### Example Request (cURL)
```bash
curl "http://127.0.0.1:8888/search?q=rust+async&format=json&categories=it&language=en"
```

### Example Request (Python)
```python
import requests

params = {
    "q": "rust async programming",
    "format": "json",
    "categories": "it,code",
    "language": "en",
    "pageno": 1
}

response = requests.get("http://127.0.0.1:8888/search", params=params)
data = response.json()

for result in data["results"]:
    print(f"{result['title']} ({result['engine']})")
```

### Example Request (Bash with `jq`)
```bash
curl -s "http://127.0.0.1:8888/search?q=ai+tools&format=json" | \
  jq '.results[] | select(.score > 2) | {title, url, engine, score}'
```

---

## 3. MCP Protocol (Model Context Protocol)

Metaseek supports the MCP protocol for direct integration with AI agents and LLMs.

### Stdio Mode (Recommended for Agents)

Start the MCP server in stdio mode:

```bash
./target/release/metaseek --mcp
```

The server reads JSON-RPC requests from `stdin` and writes responses to `stdout`.

### Example MCP Request

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

### Example MCP Response

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"metaseek","version":"0.3.0"},"capabilities":{"tools":{}}}}
```

### Tools Available

**`search` Tool** - Perform web search with filters

Request:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "query": "rust programming",
      "engines": "google,duckduckgo",
      "language": "en",
      "safesearch": 1
    }
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "query": "rust programming",
    "number_of_results": 15,
    "results": [
      {
        "url": "https://www.rust-lang.org",
        "title": "Rust Programming Language",
        "content": "...",
        "engine": "google",
        "score": 2.5
      }
    ],
    "suggestions": ["rust tutorial", "rust book"],
    "answers": []
  }
}
```

### HTTP Endpoint (Experimental)

The `/mcp` endpoint accepts JSON-RPC requests via POST:

```bash
curl -X POST http://127.0.0.1:8888/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

---

## 4. JSON Response Structure

### Top-Level Keys
```json
{
  "query": "string",
  "number_of_results": 15,
  "results": [...],
  "answers": [...],
  "suggestions": [...],
  "corrections": [...],
  "infoboxes": [...],
  "unresponsive_engines": [...],
  "engine_errors": [...]
}
```

### Result Object Fields
```json
{
  "url": "https://example.com",
  "title": "Example Title",
  "content": "Snippet text...",
  "engine": "google",
  "engines": ["google", "duckduckgo"],
  "positions": [1, 3],
  "score": 2.5,
  "category": "general",
  "parsed_url": ["https", "example.com", "/", "", "", ""],
  "metadata": {
    "author": "Jane Doe",
    "published_date": "2024-01-15",
    "tags": ["rust", "programming"],
    "thumbnail": "https://example.com/thumb.jpg",
    "is_official": true
  }
}
```

### Error Handling
- **`engine_errors`**: Array of objects with `engine`, `error`, `error_type`.
- **`unresponsive_engines`**: List of engines that timed out.
- **HTTP Status**: `200` even if errors occur; check `engine_errors` in JSON.

Example error response:
```json
{
  "engine_errors": [
    {
      "engine": "linkedin_companies",
      "error": "API key required",
      "error_type": "api_key_required"
    }
  ]
}
```

---

## 5. Advanced Agent Patterns

### Pagination Loop
```bash
for page in 1 2 3; do
  curl -s "http://127.0.0.1:8888/search?q=rust&format=json&pageno=$page" | \
    jq '.results[] | .url'
done
```

### Category-Specific Search
```bash
./target/release/metaseek --query "machine learning" --categories "science,academic"
```

### Engine-Specific Search (Bang Syntax)
In CLI or API, use `!engine` in query:
```bash
./target/release/metaseek --query "!github rust-async"
```
Or via API:
```bash
curl "http://127.0.0.1:8888/search?q=!github+rust-async&format=json"
```

### Time-Range Filtering
```bash
./target/release/metaseek --query "latest news" --time_range "week"
```

### Tor/Ahmia Search (Requires Proxy)
```bash
# Set proxy
export ALL_PROXY=socks5h://127.0.0.1:9050

# Search
./target/release/metaseek --query "onion services" --engines "ahmia"
```

---

## 6. Quick Reference Card

| Task | Command |
|------|---------|
| **CLI Search** | `./metaseek --query "term"` |
| **CLI + Filter** | `--query "term" --categories "news"` |
| **API Search** | `curl "http://host/search?q=term&format=json"` |
| **API + Pagination** | `&pageno=2` |
| **API + Engine Filter** | `&engines=google,duckduckgo` |
| **API + Time Filter** | `&time_range=week` |
| **MCP Stdio** | `./metaseek --mcp` |
| **MCP HTTP** | `POST /mcp` |
| **Parse JSON** | `| jq '.results[]'` |
| **Tor Search** | `export ALL_PROXY=socks5h://127.0.0.1:9050` |

---

## 7. Troubleshooting

| Issue | Solution |
|-------|----------|
| **No results** | Check `engine_errors` in JSON; verify query syntax. |
| **Timeout** | Increase `--time_range` or reduce `num` results. |
| **API Key Error** | Configure `api_key` in `settings.yml` or pass via env. |
| **Tor Engine Fails** | Ensure `ALL_PROXY` is set and Tor is running. |
| **Wrong Format** | Always use `&format=json` in API or `--query` in CLI. |
| **MCP Connection** | Check that stdio is properly connected; use line-delimited JSON. |

---

*For full engine list and categories, see `config/settings.yml.example` or the `/stats` endpoint.*