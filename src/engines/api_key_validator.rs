//! API Key Validation System
//!
//! Provides reusable functions for validating API keys in engines that require them.
//! Supports both configuration defaults and runtime overrides.
//!
//! ## Features
//!
//! - Validates API key from config at load time
//! - Accepts API key overrides from request parameters
//! - Prioritizes runtime keys over config defaults
//! - Clear error messages for users

use std::collections::HashMap;

/// Extract API key from config or params with priority: params > config
///
/// # Parameters
/// - `config_api_key`: The API key from engine configuration (Option<String>)
/// - `params_engine_data`: Engine-specific data from request (HashMap<String, Value>)
/// - `param_name`: The parameter name to look for (e.g., "api_key")
///
/// # Returns
/// Option<String> - The API key if available, None otherwise
///
/// # Examples
/// ```
/// use std::collections::HashMap;
/// use metaseek::engines::extract_api_key;
/// let config_key = Some("config_key".to_string());
/// let mut params = HashMap::new();
/// params.insert("api_key".to_string(), serde_json::json!("param_key"));
///
/// let key = extract_api_key(&config_key, &params, "api_key");
/// assert_eq!(key, Some("param_key".to_string())); // Params take priority
/// ```
pub fn extract_api_key(
    config_api_key: &Option<String>,
    params_engine_data: &HashMap<String, serde_json::Value>,
    param_name: &str,
) -> Option<String> {
    // Priority 1: Runtime override from params
    params_engine_data
        .get(param_name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        // Priority 2: Config default
        .or_else(|| config_api_key.clone())
}

/// Extract API key from params only (for optional API key engines)
///
/// # Parameters
/// - `params_engine_data`: Engine-specific data from request
/// - `param_name`: The parameter name to look for
///
/// # Returns
/// Option<String> - The API key if provided in params, None otherwise
pub fn extract_api_key_from_params(
    params_engine_data: &HashMap<String, serde_json::Value>,
    param_name: &str,
) -> Option<String> {
    params_engine_data
        .get(param_name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Generate a clear error message for missing API keys
///
/// # Parameters
/// - `engine_name`: The engine identifier (e.g., "linkedin_companies")
/// - `config_name`: Human-readable engine name (e.g., "LinkedIn Companies")
///
/// # Returns
/// String - Formatted error message for users
///
/// # Examples
/// ```
/// use metaseek::engines::generate_api_key_config_error;
/// let msg = generate_api_key_config_error("linkedin_companies", "LinkedIn Companies");
/// assert!(msg.contains("API key"));
/// assert!(msg.contains("metaseek.yml"));
/// ```
pub fn generate_api_key_config_error(engine_name: &str, config_name: &str) -> String {
    format!(
        "Engine '{}' requires an API key. \
         Please configure 'api_key' in config/metaseek.yml for engine '{}', \
         or pass it via request parameters.",
        engine_name, config_name
    )
}

/// Generate a detailed API key configuration error with setup instructions
///
/// # Parameters
/// - `engine_name`: The engine identifier
/// - `config_name`: Human-readable engine name
/// - `required`: Whether the engine is required for search functionality
///
/// # Returns
/// String - Detailed error message with configuration instructions
///
/// # Examples
/// ```
/// use metaseek::engines::generate_api_key_config_error_with_instructions;
/// let msg = generate_api_key_config_error_with_instructions(
///     "linkedin_companies",
///     "LinkedIn Companies",
///     false
/// );
/// assert!(msg.contains("metaseek.yml"));
/// assert!(msg.contains("LinkedIn Companies"));
/// assert!(msg.contains("developer portal"));
/// ```
pub fn generate_api_key_config_error_with_instructions(
    engine_name: &str,
    config_name: &str,
    required: bool,
) -> String {
    let mut msg = format!("Engine '{}' requires an API key. ", engine_name);

    if required {
        msg.push_str("This engine is required for your search to function properly. ");
    }

    msg.push_str(&format!(
        "To configure:\n\
         1. Generate an API key from the provider's developer portal\n\
         2. Add to config/metaseek.yml:\n\
            \n\
            - name: {}\n\
              engine: {}\n\
              api_key: \"YOUR_API_KEY_HERE\"\n\
         3. Restart the server\n\
         \n\
         Alternatively, you can pass the API key at runtime via request parameters.",
        config_name, engine_name,
    ));

    msg
}

/// Check if an API key is configured and non-empty
///
/// # Parameters
/// - `config_api_key`: The API key from engine configuration
///
/// # Returns
/// bool - True if key is configured and non-empty
///
/// # Examples
/// ```
/// use metaseek::engines::api_key_is_configured;
/// assert!(api_key_is_configured(&Some("key".to_string())));
/// assert!(!api_key_is_configured(&None));
/// assert!(!api_key_is_configured(&Some("".to_string())));
/// ```
pub fn api_key_is_configured(config_api_key: &Option<String>) -> bool {
    config_api_key
        .as_ref()
        .map(|k| !k.is_empty())
        .unwrap_or(false)
}

/// Validate that an API key is present, return formatted error if not
///
/// # Parameters
/// - `config_api_key`: The API key from engine configuration
/// - `engine_name`: The engine identifier
/// - `config_name`: Human-readable engine name
///
/// # Returns
/// Result<(), String> - Ok if key present, Err with error message otherwise
///
/// # Examples
/// ```
/// use metaseek::engines::validate_api_key_present;
/// let config_key = Some("key".to_string());
/// let result = validate_api_key_present(&config_key, "test_engine", "Test Engine");
/// assert!(result.is_ok());
/// ```
pub fn validate_api_key_present(
    config_api_key: &Option<String>,
    engine_name: &str,
    config_name: &str,
) -> Result<(), String> {
    if !api_key_is_configured(config_api_key) {
        Err(generate_api_key_config_error(engine_name, config_name))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_api_key_from_params() {
        let mut params = HashMap::new();
        params.insert("api_key".to_string(), json!("param_key"));

        let key = extract_api_key_from_params(&params, "api_key");
        assert_eq!(key, Some("param_key".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_params_missing() {
        let params = HashMap::new();

        let key = extract_api_key_from_params(&params, "api_key");
        assert_eq!(key, None);
    }

    #[test]
    fn test_extract_api_key_priority() {
        let config_key = Some("config_key".to_string());
        let mut params = HashMap::new();
        params.insert("api_key".to_string(), json!("param_key"));

        let key = extract_api_key(&config_key, &params, "api_key");
        assert_eq!(key, Some("param_key".to_string())); // Params take priority
    }

    #[test]
    fn test_extract_api_key_fallback_to_config() {
        let config_key = Some("config_key".to_string());
        let params = HashMap::new();

        let key = extract_api_key(&config_key, &params, "api_key");
        assert_eq!(key, Some("config_key".to_string()));
    }

    #[test]
    fn test_api_key_is_configured() {
        assert!(api_key_is_configured(&Some("key".to_string())));
        assert!(!api_key_is_configured(&None));
        assert!(!api_key_is_configured(&Some("".to_string())));
    }

    #[test]
    fn test_generate_api_key_config_error() {
        let msg = generate_api_key_config_error("test_engine", "Test Engine");
        assert!(msg.contains("test_engine"));
        assert!(msg.contains("Test Engine"));
        assert!(msg.contains("API key"));
        assert!(msg.contains("metaseek.yml"));
    }

    #[test]
    fn test_generate_api_key_config_error_with_instructions() {
        let msg = generate_api_key_config_error_with_instructions("test", "Test", false);
        assert!(msg.contains("metaseek.yml"));
        assert!(msg.contains("api_key:"));
        assert!(msg.contains("Restart the server"));
    }

    #[test]
    fn test_validate_api_key_present_success() {
        let config_key = Some("key".to_string());
        let result = validate_api_key_present(&config_key, "test", "Test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_api_key_present_failure() {
        let config_key: Option<String> = None;
        let result = validate_api_key_present(&config_key, "test", "Test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key"));
    }
}
