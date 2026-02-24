use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tauri_plugin_aptabase::EventTracker;
use tracing::warn;

use crate::settings::get_settings;

#[derive(Debug, Clone, Copy)]
pub enum BackendAnalyticsEvent {
    AppStarted,
    AppExited,
    TranscriptionCompleted,
    TranscriptionFailed,
    ModelDownloadStarted,
    ModelDownloadCompleted,
    ModelDownloadFailed,
    FeatureUsed,
    AhaMomentReached,
}

impl BackendAnalyticsEvent {
    fn as_name(self) -> &'static str {
        match self {
            Self::AppStarted => "app_started",
            Self::AppExited => "app_exited",
            Self::TranscriptionCompleted => "transcription_completed",
            Self::TranscriptionFailed => "transcription_failed",
            Self::ModelDownloadStarted => "model_download_started",
            Self::ModelDownloadCompleted => "model_download_completed",
            Self::ModelDownloadFailed => "model_download_failed",
            Self::FeatureUsed => "feature_used",
            Self::AhaMomentReached => "aha_moment_reached",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyticsRuntimeState {
    enabled_by_runtime: bool,
}

impl AnalyticsRuntimeState {
    pub fn enabled() -> Self {
        Self {
            enabled_by_runtime: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled_by_runtime: false,
        }
    }
}

pub fn track_backend_event(
    app: &AppHandle,
    event: BackendAnalyticsEvent,
    props: Option<Map<String, Value>>,
) -> Result<(), String> {
    track_event_internal(app, event.as_name(), props)
}

pub fn track_ui_event(
    app: &AppHandle,
    event: &str,
    props: Option<HashMap<String, String>>,
) -> Result<(), String> {
    let event_name = parse_ui_event_name(event)?;
    let mapped_props = props.map(|raw_props| {
        raw_props
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect::<Map<String, Value>>()
    });

    track_event_internal(app, event_name, mapped_props)
}

fn track_event_internal(
    app: &AppHandle,
    event_name: &str,
    props: Option<Map<String, Value>>,
) -> Result<(), String> {
    if !is_analytics_enabled(app) {
        return Ok(());
    }

    let validated_props = validate_props(event_name, props)?;
    app.track_event(event_name, validated_props.map(Value::Object))
        .map_err(|error| {
            let message = format!("failed to track analytics event '{event_name}': {error}");
            warn!("{message}");
            message
        })
}

fn is_analytics_enabled(app: &AppHandle) -> bool {
    let Some(runtime_state) = app.try_state::<AnalyticsRuntimeState>() else {
        return false;
    };

    if !runtime_state.enabled_by_runtime {
        return false;
    }

    get_settings(app).share_usage_analytics
}

fn parse_ui_event_name(event: &str) -> Result<&'static str, String> {
    match event {
        "settings_opened" => Ok("settings_opened"),
        "onboarding_completed" => Ok("onboarding_completed"),
        "analytics_toggle_changed" => Ok("analytics_toggle_changed"),
        "upgrade_prompt_shown" => Ok("upgrade_prompt_shown"),
        "upgrade_prompt_action" => Ok("upgrade_prompt_action"),
        "upgrade_checkout_result" => Ok("upgrade_checkout_result"),
        _ => Err(format!("unknown UI analytics event '{event}'")),
    }
}

fn allowed_property_keys(event_name: &str) -> Option<&'static [&'static str]> {
    match event_name {
        "app_started" => Some(&[]),
        "app_exited" => Some(&[]),
        "transcription_completed" => Some(&["result", "source_action"]),
        "transcription_failed" => Some(&["stage"]),
        "model_download_started" => Some(&[]),
        "model_download_completed" => Some(&[]),
        "model_download_failed" => Some(&[]),
        "feature_used" => Some(&["feature", "entrypoint"]),
        "aha_moment_reached" => Some(&["rule", "scope"]),
        "settings_opened" => Some(&["source"]),
        "onboarding_completed" => Some(&["source"]),
        "analytics_toggle_changed" => Some(&["enabled", "source"]),
        "upgrade_prompt_shown" => Some(&["trigger", "variant"]),
        "upgrade_prompt_action" => Some(&["action", "trigger"]),
        "upgrade_checkout_result" => Some(&["result", "source"]),
        _ => None,
    }
}

fn validate_props(
    event_name: &str,
    props: Option<Map<String, Value>>,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(allowed_keys) = allowed_property_keys(event_name) else {
        return Err(format!("unknown analytics event '{event_name}'"));
    };

    let Some(props) = props else {
        return Ok(None);
    };

    let mut sanitized = Map::new();

    for (key, value) in props {
        if !allowed_keys.contains(&key.as_str()) {
            return Err(format!(
                "event '{event_name}' does not allow property '{key}'"
            ));
        }

        if is_sensitive_key(&key) {
            return Err(format!("property '{key}' is blocked by analytics policy"));
        }

        let sanitized_value = sanitize_value(&key, value)?;
        validate_value_for_event(event_name, &key, &sanitized_value)?;
        sanitized.insert(key, sanitized_value);
    }

    if sanitized.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sanitized))
    }
}

fn sanitize_value(key: &str, value: Value) -> Result<Value, String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(format!("property '{key}' cannot be empty"));
            }
            if trimmed.chars().count() > 64 {
                return Err(format!(
                    "property '{key}' exceeds 64 characters and is likely high-cardinality"
                ));
            }
            Ok(Value::String(trimmed.to_string()))
        }
        Value::Number(number) => {
            if number.as_f64().map(|f| f.is_finite()).unwrap_or(true) {
                Ok(Value::Number(number))
            } else {
                Err(format!("property '{key}' contains a non-finite number"))
            }
        }
        Value::Bool(boolean_value) => Ok(Value::Number(Number::from(if boolean_value {
            1
        } else {
            0
        }))),
        _ => Err(format!(
            "property '{key}' must be a string, number, or boolean"
        )),
    }
}

fn validate_value_for_event(event_name: &str, key: &str, value: &Value) -> Result<(), String> {
    let value_text = value.as_str();

    match (event_name, key, value_text) {
        ("transcription_completed", "result", Some("empty" | "non_empty")) => Ok(()),
        (
            "transcription_completed",
            "source_action",
            Some("transcribe" | "transcribe_with_post_process"),
        ) => Ok(()),
        ("transcription_failed", "stage", Some("transcribe")) => Ok(()),
        (
            "feature_used",
            "feature",
            Some(
                "transcribe"
                | "transcribe_with_post_process"
                | "paste_last_transcript"
                | "undo_last_transcript"
                | "refine_last_transcript"
                | "correct_text",
            ),
        ) => Ok(()),
        (
            "feature_used",
            "entrypoint",
            Some("shortcut" | "external" | "ui"),
        ) => Ok(()),
        ("aha_moment_reached", "rule", Some("v1_5_successes")) => Ok(()),
        ("aha_moment_reached", "scope", Some("all_features")) => Ok(()),
        (
            "settings_opened",
            "source",
            Some("sidebar" | "menu"),
        ) => Ok(()),
        ("onboarding_completed", "source", Some("onboarding_flow")) => Ok(()),
        (
            "analytics_toggle_changed",
            "enabled",
            Some("enabled" | "disabled"),
        ) => Ok(()),
        (
            "analytics_toggle_changed",
            "source",
            Some("settings"),
        ) => Ok(()),
        (
            "upgrade_prompt_shown",
            "trigger",
            Some("aha_moment"),
        ) => Ok(()),
        (
            "upgrade_prompt_shown",
            "variant",
            Some("v1"),
        ) => Ok(()),
        (
            "upgrade_prompt_action",
            "action",
            Some("cta_clicked" | "dismissed" | "closed"),
        ) => Ok(()),
        (
            "upgrade_prompt_action",
            "trigger",
            Some("aha_moment"),
        ) => Ok(()),
        (
            "upgrade_checkout_result",
            "result",
            Some("started" | "completed" | "failed"),
        ) => Ok(()),
        (
            "upgrade_checkout_result",
            "source",
            Some("aha_prompt" | "settings"),
        ) => Ok(()),
        (_, _, Some(_)) => Err(format!(
            "property '{key}' has an invalid value for event '{event_name}'"
        )),
        (_, _, None) => Err(format!(
            "property '{key}' for event '{event_name}' must be a string enum value"
        )),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    normalized.contains("token")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("authorization")
        || normalized.contains("cookie")
        || normalized.contains("path")
        || normalized.contains("transcript")
        || normalized.contains("prompt")
        || normalized.contains("user_id")
        || normalized == "user"
}

#[cfg(test)]
mod tests {
    use super::{sanitize_value, validate_props};
    use serde_json::{Map, Value};

    #[test]
    fn validate_props_accepts_allowlisted_ui_event_payload() {
        let mut props = Map::new();
        props.insert("source".to_string(), Value::String("sidebar".to_string()));

        let validated = validate_props("settings_opened", Some(props)).expect("valid props");
        let source = validated
            .and_then(|map| map.get("source").cloned())
            .expect("source should be present");
        assert_eq!(source, Value::String("sidebar".to_string()));
    }

    #[test]
    fn validate_props_rejects_unknown_property() {
        let mut props = Map::new();
        props.insert("unknown".to_string(), Value::String("x".to_string()));

        let error = validate_props("settings_opened", Some(props))
            .expect_err("unknown property must fail");
        assert!(error.contains("does not allow property"));
    }

    #[test]
    fn validate_props_rejects_invalid_value_for_event() {
        let mut props = Map::new();
        props.insert("enabled".to_string(), Value::String("maybe".to_string()));
        props.insert("source".to_string(), Value::String("settings".to_string()));

        let error = validate_props("analytics_toggle_changed", Some(props))
            .expect_err("invalid value must fail");
        assert!(error.contains("invalid value"));
    }

    #[test]
    fn sanitize_value_converts_bool_to_number() {
        let converted =
            sanitize_value("enabled", Value::Bool(true)).expect("bool should convert to number");
        assert_eq!(converted, Value::Number(serde_json::Number::from(1)));
    }

    #[test]
    fn validate_props_rejects_non_string_enum_values() {
        let mut props = Map::new();
        props.insert("enabled".to_string(), Value::Bool(true));
        props.insert("source".to_string(), Value::String("settings".to_string()));

        let error = validate_props("analytics_toggle_changed", Some(props))
            .expect_err("non-string enum values must fail");
        assert!(error.contains("must be a string enum value"));
    }

    #[test]
    fn validate_props_accepts_feature_used_payload() {
        let mut props = Map::new();
        props.insert(
            "feature".to_string(),
            Value::String("transcribe".to_string()),
        );
        props.insert(
            "entrypoint".to_string(),
            Value::String("shortcut".to_string()),
        );

        let validated = validate_props("feature_used", Some(props)).expect("valid props");
        assert!(validated.is_some());
    }

    #[test]
    fn validate_props_accepts_upgrade_prompt_action_payload() {
        let mut props = Map::new();
        props.insert(
            "action".to_string(),
            Value::String("dismissed".to_string()),
        );
        props.insert(
            "trigger".to_string(),
            Value::String("aha_moment".to_string()),
        );

        let validated =
            validate_props("upgrade_prompt_action", Some(props)).expect("valid prompt action");
        assert!(validated.is_some());
    }

    #[test]
    fn validate_props_rejects_invalid_feature_value() {
        let mut props = Map::new();
        props.insert("feature".to_string(), Value::String("unknown".to_string()));
        props.insert(
            "entrypoint".to_string(),
            Value::String("shortcut".to_string()),
        );

        let error = validate_props("feature_used", Some(props))
            .expect_err("invalid feature must fail");
        assert!(error.contains("invalid value"));
    }
}
