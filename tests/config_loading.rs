use metaseek::config::Settings;
use std::collections::HashMap;
use std::path::Path;

fn environment(values: &[(&str, &str)]) -> HashMap<String, String> {
    values.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn defaults_without_sources() {
    let actual = Settings::from_sources(None, HashMap::new()).unwrap();
    assert_eq!(actual.server.port, Settings::default().server.port);
    assert_eq!(actual.engines.len(), Settings::default().engines.len());
}

#[test]
fn yaml_and_nested_environment_merge() {
    let settings = Settings::from_sources(
        Some(Path::new("config/metaseek.yml.example")),
        environment(&[
            ("METASEEK_PORT", "9001"),
            ("METASEEK_SERVER__PORT", "9002"),
            ("METASEEK_GENERAL__DEBUG", "true"),
            ("METASEEK_OUTGOING__REQUEST_TIMEOUT", "12.5"),
            ("METASEEK_BRANDING_NAME", "00123"),
            ("METASEEK_SECRET_KEY", "0012345"),
        ]),
    ).unwrap();
    assert_eq!(settings.server.port, 9002);
    assert!(settings.general.debug);
    assert_eq!(settings.outgoing.request_timeout, 12.5);
    assert_eq!(settings.branding.name.as_deref(), Some("00123"));
    assert_eq!(settings.server.secret_key, "0012345");
    assert!(!settings.engines.is_empty());
}

#[test]
fn invalid_typed_value_fails() {
    assert!(Settings::from_sources(None, environment(&[("METASEEK_PORT", "not-a-port")])).is_err());
}

#[test]
fn explicit_missing_file_fails() {
    assert!(Settings::from_sources(Some(Path::new("/nonexistent/metaseek.yml")), HashMap::new()).is_err());
}

#[test]
fn yaml_file_deserializes_directly() {
    let settings = Settings::from_file("config/metaseek.yml.example").unwrap();
    assert_eq!(settings.general.instance_name, "Metaseek");
}
