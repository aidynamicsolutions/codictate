use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use tracing::debug;

pub const USER_STORE_PATH: &str = "user_store.json";
pub const USER_PROFILE_UPDATED_EVENT: &str = "user-profile-updated";

/// User profile data - separate from app settings.
/// This stores onboarding and user identity information.
#[derive(Serialize, Deserialize, Debug, Clone, Type, Default, PartialEq, Eq)]
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

    /// Whether the user has completed a real activation by successfully dictating outside setup prompts.
    #[serde(default)]
    pub onboarding_activation_completed: bool,

    /// Whether the onboarding UI has been shown at least once.
    #[serde(default)]
    pub onboarding_started: bool,

    /// Whether the user exited onboarding UI and is completing activation from the home screen.
    #[serde(default)]
    pub onboarding_home_guidance_active: bool,

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



/// Get the user profile from the store, or create a default one if it doesn't exist.
pub fn get_user_profile<R: tauri::Runtime>(app: &AppHandle<R>) -> UserProfile {
    let store = app
        .store(USER_STORE_PATH)
        .expect("Failed to initialize user store");
    parse_profile_value(store.get("profile"))
}

fn parse_profile_value(profile_value: Option<serde_json::Value>) -> UserProfile {
    if let Some(profile_value) = profile_value {
        return match serde_json::from_value::<UserProfile>(profile_value) {
            Ok(profile) => {
                debug!("Loaded user profile: {:?}", profile);
                profile
            }
            Err(e) => {
                tracing::warn!("Failed to parse user profile: {}", e);
                UserProfile::default()
            }
        };
    }
    UserProfile::default()
}

fn apply_profile_update(
    profile: &mut UserProfile,
    key: &str,
    parsed: serde_json::Value,
) -> Result<(), String> {
    match key {
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
        "onboarding_activation_completed" => {
            profile.onboarding_activation_completed = parsed.as_bool().unwrap_or(false);
        }
        "onboarding_started" => {
            profile.onboarding_started = parsed.as_bool().unwrap_or(false);
        }
        "onboarding_home_guidance_active" => {
            profile.onboarding_home_guidance_active = parsed.as_bool().unwrap_or(false);
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

    Ok(())
}

/// Write the user profile to the store.
pub fn write_user_profile<R: tauri::Runtime>(app: &AppHandle<R>, profile: UserProfile) {
    let store = app
        .store(USER_STORE_PATH)
        .expect("Failed to initialize user store");

    store.set("profile", serde_json::to_value(&profile).unwrap());
}

fn emit_user_profile_updated<R: tauri::Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(USER_PROFILE_UPDATED_EVENT, ());
}

fn parse_onboarding_activation_surface(surface: &str) -> Result<&'static str, String> {
    match surface.trim() {
        "learn_mock_chat" => Ok("learn_mock_chat"),
        "post_onboarding" => Ok("post_onboarding"),
        other => Err(format!("unsupported onboarding activation surface '{other}'")),
    }
}

pub fn record_onboarding_activation(app: &AppHandle, surface: &str) -> Result<bool, String> {
    let surface_value = parse_onboarding_activation_surface(surface)?;

    crate::backup_restore::with_write_permit(app, || {
        let mut profile = get_user_profile(app);
        if profile.onboarding_activation_completed {
            return Ok(false);
        }

        profile.onboarding_activation_completed = true;
        profile.onboarding_home_guidance_active = false;
        write_user_profile(app, profile);
        emit_user_profile_updated(app);
        debug!("onboarding activation recorded for surface={}", surface_value);

        Ok(true)
    })
}

pub fn mark_onboarding_started(app: &AppHandle) -> Result<bool, String> {
    crate::backup_restore::with_write_permit(app, || {
        let mut profile = get_user_profile(app);
        if profile.onboarding_started {
            return Ok(false);
        }

        profile.onboarding_started = true;
        write_user_profile(app, profile);
        emit_user_profile_updated(app);

        Ok(true)
    })
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
    update_user_profile_setting_inner(&app, key, value)
}

#[tauri::command]
#[specta::specta]
pub fn record_onboarding_activation_command(
    app: AppHandle,
    surface: String,
) -> Result<bool, String> {
    record_onboarding_activation(&app, &surface)
}

#[tauri::command]
#[specta::specta]
pub fn mark_onboarding_started_command(app: AppHandle) -> Result<bool, String> {
    mark_onboarding_started(&app)
}

pub(crate) fn update_user_profile_setting_inner<R: tauri::Runtime>(
    app: &AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    crate::backup_restore::with_write_permit(app, || {
        let mut profile = get_user_profile(app);

        // Parse the JSON value
        let parsed: serde_json::Value =
            serde_json::from_str(&value).map_err(|e| format!("Invalid JSON value: {}", e))?;

        apply_profile_update(&mut profile, key.as_str(), parsed)?;

        write_user_profile(app, profile);
        debug!("Updated user profile field '{}' to: {}", key, value);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_profile_value_returns_default_for_missing_payload() {
        let profile = parse_profile_value(None);
        assert_eq!(profile, UserProfile::default());
    }

    #[test]
    fn parse_profile_value_returns_default_for_malformed_payload() {
        let malformed = Some(json!("malformed"));
        let profile = parse_profile_value(malformed);
        assert_eq!(profile, UserProfile::default());
    }

    #[test]
    fn apply_profile_update_repairs_default_profile() {
        let mut profile = parse_profile_value(Some(json!("malformed")));
        let parsed = serde_json::from_str::<serde_json::Value>("\"Ari\"")
            .expect("parse profile value");
        apply_profile_update(&mut profile, "user_name", parsed).expect("apply profile update");
        assert_eq!(profile.user_name.as_deref(), Some("Ari"));
    }

    #[test]
    fn apply_profile_update_supports_home_guidance_flag() {
        let mut profile = UserProfile::default();
        apply_profile_update(&mut profile, "onboarding_home_guidance_active", json!(true))
            .expect("apply home guidance flag");
        assert!(profile.onboarding_home_guidance_active);
    }

    #[test]
    fn apply_profile_update_supports_onboarding_started_flag() {
        let mut profile = UserProfile::default();
        apply_profile_update(&mut profile, "onboarding_started", json!(true))
            .expect("apply onboarding started flag");
        assert!(profile.onboarding_started);
    }

    #[test]
    fn apply_profile_update_supports_onboarding_activation_flag() {
        let mut profile = UserProfile::default();
        apply_profile_update(&mut profile, "onboarding_activation_completed", json!(true))
            .expect("apply onboarding activation flag");
        assert!(profile.onboarding_activation_completed);
    }

    #[test]
    fn apply_profile_update_rejects_unknown_keys() {
        let mut profile = UserProfile::default();
        let error = apply_profile_update(&mut profile, "unknown_key", json!(true))
            .expect_err("unknown profile key should be rejected");
        assert!(error.contains("Unknown user profile key"));
    }
}
