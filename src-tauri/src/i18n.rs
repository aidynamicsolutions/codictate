//! Internationalization (i18n) support for Rust-side translated strings.
//!
//! This module loads translation files from the resources directory and provides
//! a simple API to get translated strings based on the current app language.

use log::{debug, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

use crate::settings::get_settings;

/// Global cache for loaded translations
/// Key: language code (e.g., "en", "es", "fr")
/// Value: nested JSON object with translations
static TRANSLATIONS: RwLock<Option<HashMap<String, Value>>> = RwLock::new(None);

/// Load translations from the embedded resources
fn load_translations(app: &AppHandle) -> HashMap<String, Value> {
    let mut translations = HashMap::new();
    
    // List of supported languages (matching src/i18n/locales/)
    let languages = ["en", "es", "fr", "vi", "de", "it", "ja", "pl", "ru", "zh"];
    
    for lang in languages {
        let path = format!("resources/locales/{}/translation.json", lang);
        
        match app.path().resolve(&path, tauri::path::BaseDirectory::Resource) {
            Ok(file_path) => {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        match serde_json::from_str::<Value>(&content) {
                            Ok(json) => {
                                debug!("Loaded translations for language: {}", lang);
                                translations.insert(lang.to_string(), json);
                            }
                            Err(e) => {
                                warn!("Failed to parse translation file for {}: {}", lang, e);
                            }
                        }
                    }
                    Err(e) => {
                        // Only warn for non-English languages since they might not exist yet
                        if lang == "en" {
                            warn!("Failed to read translation file for {}: {}", lang, e);
                        } else {
                            debug!("Translation file not found for {}: {}", lang, e);
                        }
                    }
                }
            }
            Err(e) => {
                if lang == "en" {
                    warn!("Failed to resolve translation path for {}: {}", lang, e);
                }
            }
        }
    }
    
    translations
}

/// Initialize the i18n system by loading all translations
pub fn init(app: &AppHandle) {
    let translations = load_translations(app);
    
    if let Ok(mut cache) = TRANSLATIONS.write() {
        *cache = Some(translations);
        debug!("i18n system initialized");
    }
}

/// Get a translated string for the given key path (e.g., "recording.limitWarning")
/// Falls back to English if the translation is not found, then to the key itself.
pub fn t(app: &AppHandle, key: &str) -> String {
    // Try to get from cache first
    {
        let cache = TRANSLATIONS.read().unwrap();
        if let Some(translations) = cache.as_ref() {
            return lookup_translation(translations, &get_settings(app).app_language, key);
        }
    }
    
    // Cache was empty, initialize and try again (non-recursive to prevent infinite loop)
    init(app);
    
    let cache = TRANSLATIONS.read().unwrap();
    match cache.as_ref() {
        Some(translations) => lookup_translation(translations, &get_settings(app).app_language, key),
        None => {
            // If still empty after init, return the key itself
            warn!("i18n cache empty after initialization, returning key: {}", key);
            key.to_string()
        }
    }
}

/// Internal helper to look up a translation from the cache
fn lookup_translation(translations: &HashMap<String, Value>, lang: &str, key: &str) -> String {
    // Try the user's language first, then fall back to English
    let languages_to_try = if lang != "en" {
        vec![lang, "en"]
    } else {
        vec!["en"]
    };
    
    for try_lang in languages_to_try {
        if let Some(lang_translations) = translations.get(try_lang) {
            if let Some(value) = get_nested_value(lang_translations, key) {
                if let Some(s) = value.as_str() {
                    return s.to_string();
                }
            }
        }
    }
    
    // If no translation found, return the key itself
    warn!("Translation not found for key: {}", key);
    key.to_string()
}

/// Navigate through nested JSON object using dot-separated key path
fn get_nested_value<'a>(json: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = json;
    
    for part in key.split('.') {
        match current.get(part) {
            Some(v) => current = v,
            None => return None,
        }
    }
    
    Some(current)
}
