//! Localization module for Metaseek
//!
//! Handles language/locale detection and translation.

use std::collections::HashMap;

/// Supported languages
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("all", "All languages"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("es", "Español"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("ja", "日本語"),
    ("zh", "中文"),
    ("ko", "한국어"),
    ("ar", "العربية"),
];

/// Right-to-left languages
pub const RTL_LANGUAGES: &[&str] = &["ar", "he", "fa", "ur"];

/// Locale information
#[derive(Debug, Clone)]
pub struct Locale {
    pub code: String,
    pub name: String,
    pub native_name: String,
    pub is_rtl: bool,
}

impl Locale {
    pub fn new(code: &str, name: &str, native_name: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            native_name: native_name.to_string(),
            is_rtl: RTL_LANGUAGES.contains(&code),
        }
    }
}

/// Get locale from language code
pub fn get_locale(code: &str) -> Option<Locale> {
    let base_code = code.split('-').next().unwrap_or(code);

    SUPPORTED_LANGUAGES
        .iter()
        .find(|(c, _)| *c == base_code)
        .map(|(c, name)| Locale::new(c, name, name))
}

/// Parse Accept-Language header and return best matching locale
pub fn parse_accept_language(header: &str) -> Option<String> {
    // Parse header like "en-US,en;q=0.9,de;q=0.8"
    let mut locales: Vec<(String, f32)> = header
        .split(',')
        .filter_map(|part| {
            let mut parts = part.trim().split(';');
            let lang = parts.next()?.trim().to_string();

            let quality = parts
                .next()
                .and_then(|q| q.trim().strip_prefix("q=").and_then(|v| v.parse().ok()))
                .unwrap_or(1.0);

            Some((lang, quality))
        })
        .collect();

    // Sort by quality descending
    locales.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Find first supported locale
    for (lang, _) in locales {
        let base = lang.split('-').next().unwrap_or(&lang);
        if SUPPORTED_LANGUAGES.iter().any(|(c, _)| *c == base) {
            return Some(base.to_string());
        }
    }

    None
}

/// Check if a language is right-to-left
pub fn is_rtl(lang: &str) -> bool {
    let base = lang.split('-').next().unwrap_or(lang);
    RTL_LANGUAGES.contains(&base)
}

/// Get list of all supported languages
pub fn get_supported_languages() -> Vec<(&'static str, &'static str)> {
    SUPPORTED_LANGUAGES.to_vec()
}

/// Simple translation store (for demo - would load from files in production)
pub struct Translations {
    translations: HashMap<String, HashMap<String, String>>,
}

impl Translations {
    pub fn new() -> Self {
        let mut translations = HashMap::new();

        // English (default)
        let mut en = HashMap::new();
        en.insert("search".to_string(), "Search".to_string());
        en.insert("preferences".to_string(), "Preferences".to_string());
        en.insert("about".to_string(), "About".to_string());
        en.insert("stats".to_string(), "Statistics".to_string());
        en.insert("no_results".to_string(), "No results found".to_string());
        translations.insert("en".to_string(), en);

        // German
        let mut de = HashMap::new();
        de.insert("search".to_string(), "Suchen".to_string());
        de.insert("preferences".to_string(), "Einstellungen".to_string());
        de.insert("about".to_string(), "Über".to_string());
        de.insert("stats".to_string(), "Statistiken".to_string());
        de.insert(
            "no_results".to_string(),
            "Keine Ergebnisse gefunden".to_string(),
        );
        translations.insert("de".to_string(), de);

        // French
        let mut fr = HashMap::new();
        fr.insert("search".to_string(), "Rechercher".to_string());
        fr.insert("preferences".to_string(), "Préférences".to_string());
        fr.insert("about".to_string(), "À propos".to_string());
        fr.insert("stats".to_string(), "Statistiques".to_string());
        fr.insert(
            "no_results".to_string(),
            "Aucun résultat trouvé".to_string(),
        );
        translations.insert("fr".to_string(), fr);

        Self { translations }
    }

    /// Get a translation for a key in the specified language
    pub fn get(&self, lang: &str, key: &str) -> Option<&str> {
        let base_lang = lang.split('-').next().unwrap_or(lang);

        self.translations
            .get(base_lang)
            .and_then(|t| t.get(key))
            .map(|s| s.as_str())
            .or_else(|| {
                // Fallback to English
                self.translations
                    .get("en")
                    .and_then(|t| t.get(key))
                    .map(|s| s.as_str())
            })
    }
}

impl Default for Translations {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_language() {
        let result = parse_accept_language("en-US,en;q=0.9,de;q=0.8");
        assert_eq!(result, Some("en".to_string()));

        let result = parse_accept_language("de-DE,de;q=0.9");
        assert_eq!(result, Some("de".to_string()));
    }

    #[test]
    fn test_rtl() {
        assert!(is_rtl("ar"));
        assert!(is_rtl("ar-SA"));
        assert!(!is_rtl("en"));
    }

    #[test]
    fn test_translations() {
        let t = Translations::new();
        assert_eq!(t.get("en", "search"), Some("Search"));
        assert_eq!(t.get("de", "search"), Some("Suchen"));
        assert_eq!(t.get("fr", "search"), Some("Rechercher"));
    }
}
