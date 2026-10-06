//! HTTP client for making requests to search engines

use super::user_agent::{accept_html, accept_language, generate_user_agent};
use crate::config::OutgoingSettings;
use crate::engines::{EngineRequest, EngineResponse, HttpMethod, RequestBody};
use anyhow::Result;
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client wrapper with Metaseek-specific configuration
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    default_timeout: Duration,
    user_agent: String,
}

impl HttpClient {
    /// Create a new HTTP client with default settings
    pub fn new() -> Result<Self> {
        Self::with_settings(&OutgoingSettings::default())
    }

    /// Create a new HTTP client with custom settings
    pub fn with_settings(settings: &OutgoingSettings) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs_f64(settings.request_timeout))
            .pool_max_idle_per_host(settings.pool_maxsize)
            .gzip(true)
            .brotli(true);

        // SSL verification
        if !settings.verify_ssl {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // Proxy settings
        if let Some(ref proxy_url) = settings.proxies.all {
            builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
        } else {
            if let Some(ref http) = settings.proxies.http {
                builder = builder.proxy(reqwest::Proxy::http(http)?);
            }
            if let Some(ref https) = settings.proxies.https {
                builder = builder.proxy(reqwest::Proxy::https(https)?);
            }
        }

        let client = builder.build()?;

        Ok(Self {
            client,
            default_timeout: Duration::from_secs_f64(settings.request_timeout),
            user_agent: generate_user_agent(),
        })
    }

    /// Execute an engine request
    pub async fn execute(&self, request: EngineRequest) -> Result<EngineResponse> {
        self.execute_with_timeout(request, self.default_timeout)
            .await
    }

    /// Execute an engine request with custom timeout
    pub async fn execute_with_timeout(
        &self,
        request: EngineRequest,
        timeout: Duration,
    ) -> Result<EngineResponse> {
        let mut req_builder = match request.method {
            HttpMethod::Get => self.client.get(&request.url),
            HttpMethod::Post => self.client.post(&request.url),
        };

        // Set timeout
        req_builder = req_builder.timeout(timeout);

        // Set default headers
        req_builder = req_builder
            .header("User-Agent", &self.user_agent)
            .header("Accept", accept_html())
            .header("Accept-Language", accept_language("en"))
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1");

        // Add custom headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add query parameters
        if !request.params.is_empty() {
            req_builder = req_builder.query(&request.params);
        }

        // Add cookies
        if !request.cookies.is_empty() {
            let cookie_str = request
                .cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            req_builder = req_builder.header("Cookie", cookie_str);
        }

        // Add body
        if let Some(body) = request.data {
            req_builder = match body {
                RequestBody::Form(data) => req_builder.form(&data),
                RequestBody::Json(json) => req_builder.json(&json),
                RequestBody::Raw(bytes) => req_builder.body(bytes),
            };
        }

        // Execute request
        let response = req_builder.send().await?;

        Self::parse_response(response).await
    }

    /// Simple GET request
    pub async fn get(&self, url: &str) -> Result<EngineResponse> {
        let request = EngineRequest::get(url);
        self.execute(request).await
    }

    /// GET request with parameters
    pub async fn get_with_params(
        &self,
        url: &str,
        params: HashMap<String, String>,
    ) -> Result<EngineResponse> {
        let mut request = EngineRequest::get(url);
        request.params = params;
        self.execute(request).await
    }

    /// Simple POST request
    pub async fn post(&self, url: &str, data: HashMap<String, String>) -> Result<EngineResponse> {
        let request = EngineRequest::post(url).form(data);
        self.execute(request).await
    }

    /// POST with JSON body
    pub async fn post_json(&self, url: &str, json: serde_json::Value) -> Result<EngineResponse> {
        let request = EngineRequest::post(url).json(json);
        self.execute(request).await
    }

    /// Parse response into EngineResponse
    async fn parse_response(response: Response) -> Result<EngineResponse> {
        let status = response.status().as_u16();
        let url = response.url().to_string();

        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        let text = response.text().await?;

        Ok(EngineResponse {
            status,
            headers,
            text,
            url,
        })
    }

    /// Get a new user agent
    pub fn rotate_user_agent(&mut self) {
        self.user_agent = generate_user_agent();
    }

    /// Get current user agent
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// Set custom user agent
    pub fn set_user_agent(&mut self, ua: String) {
        self.user_agent = ua;
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = HttpClient::new();
        assert!(client.is_ok());
    }
}
