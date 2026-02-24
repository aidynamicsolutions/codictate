use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tracing::{info, warn};
use once_cell::sync::Lazy;

use crate::analytics::{self, BackendAnalyticsEvent};
use crate::notification;
use crate::user_profile::{self, USER_STORE_PATH};

pub const UPGRADE_PROMPT_ELIGIBILITY_EVENT: &str = "upgrade-prompt-eligible";
pub const UPGRADE_PROMPT_OPEN_REQUEST_EVENT: &str = "upgrade-prompt-open-requested";

const GROWTH_STATE_STORE_KEY: &str = "growth_state";
const AHA_THRESHOLD: u32 = 5;
const UPGRADE_PROMPT_COOLDOWN_MS: u64 = 14 * 24 * 60 * 60 * 1_000;
static GROWTH_STATE_IO_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, Copy)]
pub enum FeatureName {
    Transcribe,
    TranscribeWithPostProcess,
    PasteLastTranscript,
    UndoLastTranscript,
    RefineLastTranscript,
    CorrectText,
}

impl FeatureName {
    fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::TranscribeWithPostProcess => "transcribe_with_post_process",
            Self::PasteLastTranscript => "paste_last_transcript",
            Self::UndoLastTranscript => "undo_last_transcript",
            Self::RefineLastTranscript => "refine_last_transcript",
            Self::CorrectText => "correct_text",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FeatureEntrypoint {
    Shortcut,
    External,
    Ui,
}

impl FeatureEntrypoint {
    fn as_str(self) -> &'static str {
        match self {
            Self::Shortcut => "shortcut",
            Self::External => "external",
            Self::Ui => "ui",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UpgradePromptEligibility {
    pub eligible: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrowthState {
    pub successful_feature_uses: u32,
    pub aha_reached_at_ms: Option<u64>,
    pub last_upgrade_prompt_at_ms: Option<u64>,
    pub upgrade_prompt_shown_count: u32,
    pub pending_upgrade_prompt_open: bool,
    pub is_paid: bool,
}

impl Default for GrowthState {
    fn default() -> Self {
        Self {
            successful_feature_uses: 0,
            aha_reached_at_ms: None,
            last_upgrade_prompt_at_ms: None,
            upgrade_prompt_shown_count: 0,
            pending_upgrade_prompt_open: false,
            is_paid: false,
        }
    }
}

pub fn classify_entrypoint(source_hint: &str) -> FeatureEntrypoint {
    let normalized = source_hint.trim().to_ascii_lowercase();
    if normalized == "cli" || normalized.starts_with("sigusr") || normalized == "external" {
        return FeatureEntrypoint::External;
    }
    if normalized == "ui" || normalized == "menu" || normalized == "settings" {
        return FeatureEntrypoint::Ui;
    }
    FeatureEntrypoint::Shortcut
}

pub fn record_feature_success(app: &AppHandle, feature: FeatureName, entrypoint: FeatureEntrypoint) {
    let _state_guard = lock_growth_state_io();
    let mut state = load_growth_state(app);
    let now = now_ms();
    let aha_reached_now = apply_feature_success(&mut state, now);

    let mut feature_props = serde_json::Map::new();
    feature_props.insert(
        "feature".to_string(),
        serde_json::Value::String(feature.as_str().to_string()),
    );
    feature_props.insert(
        "entrypoint".to_string(),
        serde_json::Value::String(entrypoint.as_str().to_string()),
    );
    let _ = analytics::track_backend_event(
        app,
        BackendAnalyticsEvent::FeatureUsed,
        Some(feature_props),
    );

    if aha_reached_now {
        let mut aha_props = serde_json::Map::new();
        aha_props.insert(
            "rule".to_string(),
            serde_json::Value::String("v1_5_successes".to_string()),
        );
        aha_props.insert(
            "scope".to_string(),
            serde_json::Value::String("all_features".to_string()),
        );
        let _ = analytics::track_backend_event(
            app,
            BackendAnalyticsEvent::AhaMomentReached,
            Some(aha_props),
        );
    }

    let onboarding_completed = user_profile::get_user_profile(app).onboarding_completed;
    let eligibility = evaluate_upgrade_prompt_eligibility_at(&state, onboarding_completed, now);
    maybe_dispatch_upgrade_prompt_nudge(app, &mut state, &eligibility, now);
    persist_growth_state(app, &state);
}

pub fn get_upgrade_prompt_eligibility(app: &AppHandle) -> UpgradePromptEligibility {
    let _state_guard = lock_growth_state_io();
    let state = load_growth_state(app);
    let onboarding_completed = user_profile::get_user_profile(app).onboarding_completed;
    evaluate_upgrade_prompt_eligibility_at(&state, onboarding_completed, now_ms())
}

pub fn mark_upgrade_prompt_shown(
    app: &AppHandle,
    trigger: &str,
    variant: &str,
) -> Result<(), String> {
    let _state_guard = lock_growth_state_io();
    let trigger_value = parse_upgrade_prompt_trigger(trigger)?;
    let variant_value = parse_upgrade_prompt_variant(variant)?;

    let mut state = load_growth_state(app);
    state.last_upgrade_prompt_at_ms = Some(now_ms());
    state.upgrade_prompt_shown_count = state.upgrade_prompt_shown_count.saturating_add(1);
    state.pending_upgrade_prompt_open = false;
    persist_growth_state(app, &state);

    let props = HashMap::from([
        ("trigger".to_string(), trigger_value.to_string()),
        ("variant".to_string(), variant_value.to_string()),
    ]);
    analytics::track_ui_event(app, "upgrade_prompt_shown", Some(props))
}

pub fn mark_upgrade_prompt_action(
    app: &AppHandle,
    action: &str,
    trigger: &str,
) -> Result<(), String> {
    let action_value = parse_upgrade_prompt_action(action)?;
    let trigger_value = parse_upgrade_prompt_trigger(trigger)?;

    let props = HashMap::from([
        ("action".to_string(), action_value.to_string()),
        ("trigger".to_string(), trigger_value.to_string()),
    ]);
    analytics::track_ui_event(app, "upgrade_prompt_action", Some(props))
}

pub fn mark_upgrade_checkout_result(
    app: &AppHandle,
    result: &str,
    source: &str,
) -> Result<(), String> {
    let _state_guard = lock_growth_state_io();
    let result_value = parse_upgrade_checkout_result(result)?;
    let source_value = parse_upgrade_checkout_source(source)?;

    if result_value == "completed" {
        let mut state = load_growth_state(app);
        state.is_paid = true;
        persist_growth_state(app, &state);
    }

    let props = HashMap::from([
        ("result".to_string(), result_value.to_string()),
        ("source".to_string(), source_value.to_string()),
    ]);
    analytics::track_ui_event(app, "upgrade_checkout_result", Some(props))
}

pub fn consume_pending_upgrade_prompt_open_request(app: &AppHandle) -> bool {
    let _state_guard = lock_growth_state_io();
    let mut state = load_growth_state(app);
    if !state.pending_upgrade_prompt_open {
        return false;
    }

    state.pending_upgrade_prompt_open = false;
    persist_growth_state(app, &state);
    true
}

pub fn has_pending_upgrade_prompt_open_request(app: &AppHandle) -> bool {
    let _state_guard = lock_growth_state_io();
    load_growth_state(app).pending_upgrade_prompt_open
}

fn parse_upgrade_prompt_trigger(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "aha_moment" => Ok("aha_moment"),
        other => Err(format!("unsupported upgrade prompt trigger '{other}'")),
    }
}

fn parse_upgrade_prompt_variant(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "v1" => Ok("v1"),
        other => Err(format!("unsupported upgrade prompt variant '{other}'")),
    }
}

fn parse_upgrade_prompt_action(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "cta_clicked" => Ok("cta_clicked"),
        "dismissed" => Ok("dismissed"),
        "closed" => Ok("closed"),
        other => Err(format!("unsupported upgrade prompt action '{other}'")),
    }
}

fn parse_upgrade_checkout_result(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "started" => Ok("started"),
        "completed" => Ok("completed"),
        "failed" => Ok("failed"),
        other => Err(format!("unsupported checkout result '{other}'")),
    }
}

fn parse_upgrade_checkout_source(value: &str) -> Result<&'static str, String> {
    match value.trim() {
        "aha_prompt" => Ok("aha_prompt"),
        "settings" => Ok("settings"),
        other => Err(format!("unsupported checkout source '{other}'")),
    }
}

fn lock_growth_state_io() -> MutexGuard<'static, ()> {
    GROWTH_STATE_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| {
            warn!("growth state mutex poisoned; recovering");
            poisoned.into_inner()
        })
}

fn load_growth_state(app: &AppHandle) -> GrowthState {
    let store = match app.store(USER_STORE_PATH) {
        Ok(store) => store,
        Err(error) => {
            warn!("failed to initialize user store for growth state: {error}");
            return GrowthState::default();
        }
    };

    let Some(stored_value) = store.get(GROWTH_STATE_STORE_KEY) else {
        return GrowthState::default();
    };

    match serde_json::from_value::<GrowthState>(stored_value) {
        Ok(state) => state,
        Err(error) => {
            warn!("failed to parse growth state from user store: {error}");
            GrowthState::default()
        }
    }
}

fn persist_growth_state(app: &AppHandle, state: &GrowthState) {
    let Ok(store) = app.store(USER_STORE_PATH) else {
        return;
    };

    if let Ok(value) = serde_json::to_value(state) {
        store.set(GROWTH_STATE_STORE_KEY, value);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn apply_feature_success(state: &mut GrowthState, now: u64) -> bool {
    state.successful_feature_uses = state.successful_feature_uses.saturating_add(1);
    if state.aha_reached_at_ms.is_none() && state.successful_feature_uses >= AHA_THRESHOLD {
        state.aha_reached_at_ms = Some(now);
        return true;
    }
    false
}

fn evaluate_upgrade_prompt_eligibility_at(
    state: &GrowthState,
    onboarding_completed: bool,
    now: u64,
) -> UpgradePromptEligibility {
    if state.aha_reached_at_ms.is_none() {
        return UpgradePromptEligibility {
            eligible: false,
            reason: "aha_not_reached".to_string(),
        };
    }

    if state.is_paid {
        return UpgradePromptEligibility {
            eligible: false,
            reason: "paid_user".to_string(),
        };
    }

    if !onboarding_completed {
        return UpgradePromptEligibility {
            eligible: false,
            reason: "onboarding_incomplete".to_string(),
        };
    }

    if let Some(last_prompt_at) = state.last_upgrade_prompt_at_ms {
        if now.saturating_sub(last_prompt_at) < UPGRADE_PROMPT_COOLDOWN_MS {
            return UpgradePromptEligibility {
                eligible: false,
                reason: "cooldown_active".to_string(),
            };
        }
    }

    UpgradePromptEligibility {
        eligible: true,
        reason: "eligible".to_string(),
    }
}

fn emit_upgrade_prompt_eligible_event(app: &AppHandle, eligibility: &UpgradePromptEligibility) -> bool {
    if !eligibility.eligible {
        return false;
    }

    app.emit(UPGRADE_PROMPT_ELIGIBILITY_EVENT, eligibility).is_ok()
}

fn maybe_dispatch_upgrade_prompt_nudge(
    app: &AppHandle,
    state: &mut GrowthState,
    eligibility: &UpgradePromptEligibility,
    now: u64,
) {
    if !eligibility.eligible {
        return;
    }

    let (window_visible, window_focused) = main_window_visibility_and_focus(app);

    if window_visible && window_focused {
        info!(
            event_code = "growth_nudge_decision",
            action = "emit_in_app_banner",
            window_visible,
            window_focused,
            "Aha nudge routed to in-app banner"
        );
        if emit_upgrade_prompt_eligible_event(app, eligibility) {
            state.last_upgrade_prompt_at_ms = Some(now);
            state.pending_upgrade_prompt_open = false;
            return;
        }
        warn!(
            event_code = "growth_nudge_decision",
            action = "emit_in_app_banner_failed_fallback_notification",
            "Failed to emit in-app banner event; falling back to system notification"
        );
    }

    info!(
        event_code = "growth_nudge_decision",
        action = "show_system_notification",
        window_visible,
        window_focused,
        "Aha nudge routed to system notification"
    );
    state.pending_upgrade_prompt_open = true;
    state.last_upgrade_prompt_at_ms = Some(now);
    notification::show_upgrade_prompt_nudge(app);
}

fn main_window_visibility_and_focus(app: &AppHandle) -> (bool, bool) {
    let Some(window) = app.get_webview_window("main") else {
        return (false, false);
    };

    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    (visible, focused)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_feature_success, classify_entrypoint, evaluate_upgrade_prompt_eligibility_at,
        FeatureEntrypoint, GrowthState,
    };

    #[test]
    fn aha_reaches_once_at_threshold() {
        let mut state = GrowthState::default();

        for index in 0..4 {
            let reached = apply_feature_success(&mut state, 1_000 + index);
            assert!(!reached);
        }

        assert!(apply_feature_success(&mut state, 2_000));
        assert_eq!(state.aha_reached_at_ms, Some(2_000));

        assert!(!apply_feature_success(&mut state, 3_000));
        assert_eq!(state.aha_reached_at_ms, Some(2_000));
    }

    #[test]
    fn eligibility_requires_onboarding_completion() {
        let mut state = GrowthState::default();
        state.aha_reached_at_ms = Some(1_000);

        let result = evaluate_upgrade_prompt_eligibility_at(&state, false, 2_000);
        assert!(!result.eligible);
        assert_eq!(result.reason, "onboarding_incomplete");
    }

    #[test]
    fn eligibility_respects_cooldown() {
        let mut state = GrowthState::default();
        state.aha_reached_at_ms = Some(1_000);
        state.last_upgrade_prompt_at_ms = Some(10_000);

        let within_cooldown = evaluate_upgrade_prompt_eligibility_at(
            &state,
            true,
            10_000 + (13 * 24 * 60 * 60 * 1_000),
        );
        assert!(!within_cooldown.eligible);
        assert_eq!(within_cooldown.reason, "cooldown_active");

        let after_cooldown = evaluate_upgrade_prompt_eligibility_at(
            &state,
            true,
            10_000 + (14 * 24 * 60 * 60 * 1_000),
        );
        assert!(after_cooldown.eligible);
        assert_eq!(after_cooldown.reason, "eligible");
    }

    #[test]
    fn paid_state_suppresses_prompt_eligibility() {
        let mut state = GrowthState::default();
        state.aha_reached_at_ms = Some(1_000);
        state.is_paid = true;

        let result = evaluate_upgrade_prompt_eligibility_at(&state, true, 2_000);
        assert!(!result.eligible);
        assert_eq!(result.reason, "paid_user");
    }

    #[test]
    fn classify_entrypoint_maps_known_sources() {
        assert!(matches!(
            classify_entrypoint("CLI"),
            FeatureEntrypoint::External
        ));
        assert!(matches!(
            classify_entrypoint("SIGUSR1"),
            FeatureEntrypoint::External
        ));
        assert!(matches!(
            classify_entrypoint("menu"),
            FeatureEntrypoint::Ui
        ));
        assert!(matches!(
            classify_entrypoint("ctrl+space"),
            FeatureEntrypoint::Shortcut
        ));
    }
}
