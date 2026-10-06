//! User agent generation

use rand::seq::IndexedRandom;
use rand::RngExt;

/// Generate a random but realistic user agent string
pub fn generate_user_agent() -> String {
    let mut rng = rand::rng();

    // Chrome versions (recent)
    let chrome_versions = [
        "120.0.0.0",
        "121.0.0.0",
        "122.0.0.0",
        "123.0.0.0",
        "124.0.0.0",
        "125.0.0.0",
    ];

    // Firefox versions (recent)
    let firefox_versions = ["121.0", "122.0", "123.0", "124.0", "125.0"];

    // Safari versions
    let safari_versions = ["17.2", "17.3", "17.4"];

    // Operating systems
    let os_strings = [
        "Windows NT 10.0; Win64; x64",
        "Windows NT 11.0; Win64; x64",
        "Macintosh; Intel Mac OS X 10_15_7",
        "Macintosh; Intel Mac OS X 14_2_1",
        "X11; Linux x86_64",
        "X11; Ubuntu; Linux x86_64",
    ];

    let os = os_strings.choose(&mut rng).unwrap();

    // Browser choice
    let browser_type: u8 = rng.random_range(0..10);

    if browser_type < 6 {
        // Chrome (60% chance)
        let chrome = chrome_versions.choose(&mut rng).unwrap();
        format!(
            "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
            os, chrome
        )
    } else if browser_type < 9 {
        // Firefox (30% chance)
        let firefox = firefox_versions.choose(&mut rng).unwrap();
        format!(
            "Mozilla/5.0 ({}; rv:{}) Gecko/20100101 Firefox/{}",
            os, firefox, firefox
        )
    } else {
        // Safari (10% chance) - only on Mac
        let safari = safari_versions.choose(&mut rng).unwrap();
        format!(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_2_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{} Safari/605.1.15",
            safari
        )
    }
}

/// Standard accept headers for HTML requests
pub fn accept_html() -> &'static str {
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
}

/// Standard accept headers for JSON requests
#[allow(dead_code)]
pub fn accept_json() -> &'static str {
    "application/json,text/javascript,*/*;q=0.01"
}

/// Standard accept-language header
pub fn accept_language(lang: &str) -> String {
    if lang == "all" || lang.is_empty() {
        "en-US,en;q=0.9".to_string()
    } else {
        format!("{},en-US;q=0.9,en;q=0.8", lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_agent() {
        let ua = generate_user_agent();
        assert!(ua.starts_with("Mozilla/5.0"));
        assert!(ua.len() > 50);
    }
}
