use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tracing::debug;

pub const USER_STORE_PATH: &str = "user_store.json";

/// User profile data - separate from app settings.
/// This stores onboarding and user identity information.
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct UserProfile {
    /// User's display name (collected during onboarding)
    #[serde(default)]
    pub user_name: Option<String>,

    /// Current onboarding step (1-based index)
    #[serde(default)]
    pub onboarding_step: u8,

    /// Whether onboarding has been completed
    #[serde(default)]
    pub onboarding_completed: bool,

    /// How the user heard about the app (single source stored as array for compat)
    #[serde(default)]
    pub referral_sources: Vec<String>,

    /// Secondary details for referral source (e.g., which social media platform)
    #[serde(default)]
    pub referral_details: HashMap<String, Vec<String>>,

    /// User's work role/profession
    #[serde(default)]
    pub work_role: Option<String>,

    /// Custom text when "other" work role is selected
    #[serde(default)]
    pub work_role_other: Option<String>,

    /// Professional level (Executive, Director, Manager, etc.)
    #[serde(default)]
    pub professional_level: Option<String>,

    /// Typing use cases (multi-select from onboarding)
    #[serde(default)]
    pub typing_use_cases: Vec<String>,

    /// Custom text when "other" typing use case is selected
    #[serde(default)]
    pub typing_use_cases_other: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            user_name: None,
            onboarding_step: 0,
            onboarding_completed: false,
            referral_sources: Vec::new(),
            referral_details: HashMap::new(),
            work_role: None,
            work_role_other: None,
            professional_level: None,
            typing_use_cases: Vec::new(),
            typing_use_cases_other: None,
        }
    }
}

/// Get the user profile from the store, or create a default one if it doesn't exist.
pub fn get_user_profile(app: &AppHandle) -> UserProfile {
    let store = app
        .store(USER_STORE_PATH)
        .expect("Failed to initialize user store");

    if let Some(profile_value) = store.get("profile") {
        match serde_json::from_value::<UserProfile>(profile_value) {
            Ok(profile) => {
                debug!("Loaded user profile: {:?}", profile);
                profile
            }
            Err(e) => {
                tracing::warn!("Failed to parse user profile: {}", e);
                let default_profile = UserProfile::default();
                store.set(
                    "profile",
                    serde_json::to_value(&default_profile).unwrap(),
                );
                default_profile
            }
        }
    } else {
        let default_profile = UserProfile::default();
        store.set(
            "profile",
            serde_json::to_value(&default_profile).unwrap(),
        );
        default_profile
    }
}

/// Write the user profile to the store.
pub fn write_user_profile(app: &AppHandle, profile: UserProfile) {
    let store = app
        .store(USER_STORE_PATH)
        .expect("Failed to initialize user store");

    store.set("profile", serde_json::to_value(&profile).unwrap());
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
#[specta::specta]
pub fn get_user_profile_command(app: AppHandle) -> Result<UserProfile, String> {
    Ok(get_user_profile(&app))
}

/// Update a specific field in the user profile.
/// The value is a JSON-encoded string.
#[tauri::command]
#[specta::specta]
pub fn update_user_profile_setting(
    app: AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut profile = get_user_profile(&app);

    // Parse the JSON value
    let parsed: serde_json::Value =
        serde_json::from_str(&value).map_err(|e| format!("Invalid JSON value: {}", e))?;

    match key.as_str() {
        "user_name" => {
            profile.user_name = if parsed.is_null() {
                None
            } else {
                parsed.as_str().map(String::from)
            };
        }
        "onboarding_step" => {
            profile.onboarding_step = parsed.as_u64().unwrap_or(0) as u8;
        }
        "onboarding_completed" => {
            profile.onboarding_completed = parsed.as_bool().unwrap_or(false);
        }
        "referral_sources" => {
            profile.referral_sources = serde_json::from_value(parsed).unwrap_or_default();
        }
        "referral_details" => {
            profile.referral_details = serde_json::from_value(parsed).unwrap_or_default();
        }
        "work_role" => {
            profile.work_role = if parsed.is_null() {
                None
            } else {
                parsed.as_str().map(String::from)
            };
        }
        "work_role_other" => {
            profile.work_role_other = if parsed.is_null() {
                None
            } else {
                parsed.as_str().map(String::from)
            };
        }
        "professional_level" => {
            profile.professional_level = if parsed.is_null() {
                None
            } else {
                parsed.as_str().map(String::from)
            };
        }
        "typing_use_cases" => {
            profile.typing_use_cases = serde_json::from_value(parsed).unwrap_or_default();
        }
        "typing_use_cases_other" => {
            profile.typing_use_cases_other = if parsed.is_null() {
                None
            } else {
                parsed.as_str().map(String::from)
            };
        }
        _ => {
            return Err(format!("Unknown user profile key: {}", key));
        }
    }

    write_user_profile(&app, profile);
    debug!("Updated user profile field '{}' to: {}", key, value);
    Ok(())
}
