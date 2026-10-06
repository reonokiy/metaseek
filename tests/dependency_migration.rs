use metaseek::{network::HttpClient, web::Templates};
use std::collections::HashMap;
use wiremock::{matchers::{method, path, query_param, body_string_contains}, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn http_query_and_form_survive_reqwest_upgrade() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/search"))
        .and(query_param("q", "rust language"))
        .respond_with(ResponseTemplate::new(200).set_body_string("query-ok"))
        .expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/search"))
        .and(body_string_contains("q=rust+language"))
        .respond_with(ResponseTemplate::new(200).set_body_string("form-ok"))
        .expect(1).mount(&server).await;
    let client = HttpClient::new().unwrap();
    let params = HashMap::from([("q".to_string(), "rust language".to_string())]);
    let url = format!("{}/search", server.uri());
    assert_eq!(client.get_with_params(&url, params.clone()).await.unwrap().text, "query-ok");
    assert_eq!(client.post(&url, params).await.unwrap().text, "form-ok");
}

#[test]
fn embedded_templates_compile_with_tera_2() {
    Templates::new().expect("all embedded templates must compile");
}

#[test]
fn cache_key_retains_sha256_encoding() {
    use sha2::{Digest, Sha256};
    let expected: String = Sha256::digest(b"rustbing1en")
        .iter().map(|byte| format!("{byte:02x}")).collect();
    let actual = metaseek::cache::query_cache_key("rust", &["bing".to_string()], 1, "en");
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 64);
}
