use crate::input::{self, EnigoState};
use crate::i18n;
use crate::managers::audio::AudioRecordingManager;
use crate::managers::history::{HistoryManager, StatsContribution};
use crate::managers::transcription::TranscriptionManager;
use crate::notification;
use crate::overlay;
use crate::settings::{get_settings, SETTINGS_STORE_PATH};
use crate::utils;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;
use tracing::{info, warn};

const RECENT_PASTE_TTL_MS: u64 = 120_000;
const UNDO_MODIFIER_RELEASE_DELAY_MS: u64 = 350;
const STOP_TRANSITION_WINDOW_MS: u64 = 500;
const DISCOVERABILITY_HINT_DELAY_MS: u64 = 2_500;
const UNDO_DISCOVERABILITY_STORE_KEY: &str = "undo_discoverability";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentPasteSlot {
    pub paste_id: u64,
    pub source_action: String,
    pub stats_token: Option<u64>,
    pub auto_refined: bool,
    pub pasted_text: String,
    pub suggestion_text: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub consumed: bool,
}

impl RecentPasteSlot {
    fn is_expired(&self, now: u64) -> bool {
        now > self.expires_at_ms
    }

    fn is_valid(&self, now: u64) -> bool {
        !self.consumed && !self.is_expired(now)
    }

    fn consume(&mut self) {
        self.consumed = true;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UndoNudgeEvidence {
    pub has_seen_undo_hint: bool,
    pub successful_paste_count: u32,
    pub has_used_undo: bool,
}

impl Default for UndoNudgeEvidence {
    fn default() -> Self {
        Self {
            has_seen_undo_hint: false,
            successful_paste_count: 0,
            has_used_undo: false,
        }
    }
}

#[derive(Debug, Clone)]
struct TrackedStatsContribution {
    contribution: StatsContribution,
    inserted_at_ms: u64,
}

#[derive(Debug)]
pub struct UndoManager {
    recent_slot: Mutex<Option<RecentPasteSlot>>,
    paste_id_counter: AtomicU64,
    stats_token_counter: AtomicU64,
    stats_contributions: Mutex<HashMap<u64, TrackedStatsContribution>>,
    pending_stats_rollbacks: Mutex<HashSet<u64>>,
    stop_transition_started_ms: Mutex<Option<u64>>,
    #[cfg(target_os = "linux")]
    pending_linux_toast: Mutex<Option<UndoUiEvent>>,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self {
            recent_slot: Mutex::new(None),
            paste_id_counter: AtomicU64::new(1),
            stats_token_counter: AtomicU64::new(1),
            stats_contributions: Mutex::new(HashMap::new()),
            pending_stats_rollbacks: Mutex::new(HashSet::new()),
            stop_transition_started_ms: Mutex::new(None),
            #[cfg(target_os = "linux")]
            pending_linux_toast: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PasteCapture {
    pub source_action: &'static str,
    pub stats_token: Option<u64>,
    pub auto_refined: bool,
    pub pasted_text: String,
    pub suggestion_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoUiKind {
    Feedback,
    DiscoverabilityHint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoUiEvent {
    pub kind: UndoUiKind,
    pub code: String,
    pub shortcut: Option<String>,
}

pub struct StopTransitionGuard {
    app: AppHandle,
}

impl StopTransitionGuard {
    pub fn new(app: &AppHandle) -> Self {
        Self { app: app.clone() }
    }
}

impl Drop for StopTransitionGuard {
    fn drop(&mut self) {
        clear_stop_transition_marker(&self.app);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn compact_shortcut_for_display(binding: &str) -> String {
    let parts: Vec<&str> = binding
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return binding.to_string();
    }

    let mapped: Vec<String> = parts
        .iter()
        .map(|part| match part.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => "ctrl".to_string(),
            "command" | "cmd" | "meta" => "cmd".to_string(),
            "option" | "opt" | "alt" => "alt".to_string(),
            "shift" => "shift".to_string(),
            "super" | "win" => "win".to_string(),
            value => value.to_string(),
        })
        .collect();

    mapped.join("+")
}

fn short_undo_binding(app: &AppHandle) -> String {
    let binding = get_settings(app)
        .bindings
        .get("undo_last_transcript")
        .map(|binding| binding.current_binding.clone())
        .unwrap_or_else(|| {
            #[cfg(target_os = "macos")]
            {
                "control+command+z".to_string()
            }
            #[cfg(not(target_os = "macos"))]
            {
                "ctrl+alt+z".to_string()
            }
        });

    compact_shortcut_for_display(&binding)
}

fn is_fn_based_binding(binding: &str) -> bool {
    let normalized = binding.to_ascii_lowercase();
    normalized == "fn" || normalized.starts_with("fn+") || normalized.contains("+fn")
}

fn is_undo_shortcut_available(app: &AppHandle) -> bool {
    let Some(binding) = get_settings(app)
        .bindings
        .get("undo_last_transcript")
        .map(|binding| binding.current_binding.trim().to_string())
    else {
        warn!(
            event_code = "undo_discoverability_hint_shortcut_unavailable",
            reason = "binding_missing",
            "Discoverability shortcut availability check failed"
        );
        return false;
    };

    if binding.is_empty() {
        warn!(
            event_code = "undo_discoverability_hint_shortcut_unavailable",
            reason = "binding_empty",
            "Discoverability shortcut availability check failed"
        );
        return false;
    }

    if is_fn_based_binding(&binding) {
        return true;
    }

    let shortcut = match binding.parse::<Shortcut>() {
        Ok(shortcut) => shortcut,
        Err(error) => {
            warn!(
                event_code = "undo_discoverability_hint_shortcut_unavailable",
                reason = "binding_parse_failed",
                binding = %binding,
                error = %error,
                "Discoverability shortcut availability check failed"
            );
            return false;
        }
    };

    if app.global_shortcut().is_registered(shortcut) {
        return true;
    }

    info!(
        event_code = "undo_discoverability_hint_shortcut_unavailable",
        reason = "binding_not_registered",
        binding = %binding,
        "Discoverability shortcut availability check failed"
    );
    false
}

fn i18n_format(app: &AppHandle, key: &str, replacements: &[(&str, String)]) -> String {
    let mut value = i18n::t(app, key);
    for (name, replacement) in replacements {
        // i18next interpolation tokens are wrapped with double braces (`{{token}}`).
        let token = format!("{{{{{}}}}}", name);
        value = value.replace(&token, replacement);
    }
    value
}

fn notify_text_for_event(app: &AppHandle, event: &UndoUiEvent) -> String {
    match event.kind {
        UndoUiKind::Feedback => match event.code.as_str() {
            "undo_success" => i18n::t(app, "overlay.undo.feedback.success"),
            "undo_failed" => i18n::t(app, "overlay.undo.feedback.failed"),
            "undo_recording_canceled" => i18n::t(app, "overlay.undo.feedback.recordingCanceled"),
            "undo_processing_canceled" => i18n::t(app, "overlay.undo.feedback.processingCanceled"),
            "undo_noop_empty" => i18n::t(app, "overlay.undo.feedback.nothingToUndo"),
            "undo_noop_expired" => i18n::t(app, "overlay.undo.feedback.expired"),
            _ => i18n::t(app, "overlay.undo.feedback.success"),
        },
        UndoUiKind::DiscoverabilityHint => i18n_format(
            app,
            "overlay.undo.discoverability.hint",
            &[(
                "shortcut",
                event.shortcut.clone().unwrap_or_else(|| {
                    i18n::t(app, "settings.general.shortcut.bindings.undo_last_transcript.name")
                }),
            )],
        ),
    }
}

fn main_window_focused(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false)
}

fn emit_main_toast_event(app: &AppHandle, event: &UndoUiEvent) {
    let _ = app.emit("undo-main-toast", event);
}

fn emit_ui_event(app: &AppHandle, event: UndoUiEvent) {
    if overlay::emit_undo_overlay_event(app, &event) {
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let message = notify_text_for_event(app, &event);
        notification::show_info_with_text(app, &message);

        if let Some(manager) = app.try_state::<UndoManager>() {
            let mut pending = manager.pending_linux_toast.lock().unwrap();
            *pending = Some(event);
        }
        return;
    }

    #[cfg(not(target_os = "linux"))]
    {
        if main_window_focused(app) {
            emit_main_toast_event(app, &event);
            return;
        }

        let message = notify_text_for_event(app, &event);
        notification::show_info_with_text(app, &message);
        emit_main_toast_event(app, &event);
    }
}

pub fn flush_pending_linux_toast(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    {
        let Some(manager) = app.try_state::<UndoManager>() else {
            return;
        };

        if !main_window_focused(app) {
            return;
        }

        let mut pending = manager.pending_linux_toast.lock().unwrap();
        if let Some(event) = pending.take() {
            emit_main_toast_event(app, &event);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Linux-only code path above consumes `app`; keep signature consistent across targets.
        let _ = app;
    }
}

fn load_evidence(app: &AppHandle) -> UndoNudgeEvidence {
    let store = match app.store(SETTINGS_STORE_PATH) {
        Ok(store) => store,
        Err(error) => {
            warn!(
                event_code = "undo_discoverability_store_open_failed",
                error = %error,
                "Failed to open discoverability store"
            );
            return UndoNudgeEvidence::default();
        }
    };

    if let Some(value) = store.get(UNDO_DISCOVERABILITY_STORE_KEY) {
        match serde_json::from_value::<UndoNudgeEvidence>(value) {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!(
                    event_code = "undo_discoverability_store_parse_failed",
                    error = %error,
                    "Failed to parse discoverability store payload"
                );
                UndoNudgeEvidence::default()
            }
        }
    } else {
        UndoNudgeEvidence::default()
    }
}

fn persist_evidence(app: &AppHandle, evidence: &UndoNudgeEvidence) {
    let store = match app.store(SETTINGS_STORE_PATH) {
        Ok(store) => store,
        Err(error) => {
            warn!(
                event_code = "undo_discoverability_store_open_failed",
                error = %error,
                "Failed to open discoverability store for write"
            );
            return;
        }
    };

    store.set(
        UNDO_DISCOVERABILITY_STORE_KEY,
        serde_json::to_value(evidence).unwrap_or_else(|_| serde_json::json!({})),
    );
    if let Err(error) = store.save() {
        warn!(
            event_code = "undo_discoverability_store_save_failed",
            error = %error,
            "Failed to save discoverability store payload"
        );
    }
}

fn discoverability_skip_reason(
    evidence: &UndoNudgeEvidence,
    shortcut_available: bool,
) -> Option<&'static str> {
    if evidence.has_seen_undo_hint {
        return Some("has_seen");
    }

    if evidence.has_used_undo {
        return Some("has_used_undo");
    }

    if evidence.successful_paste_count < 2 {
        return Some("insufficient_paste_count");
    }

    if !shortcut_available {
        return Some("undo_shortcut_unavailable");
    }

    None
}

fn platform_undo_key_path() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "meta+other(6)"
    }
    #[cfg(target_os = "windows")]
    {
        "control+other(0x5A)"
    }
    #[cfg(target_os = "linux")]
    {
        "control+unicode(z)"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "control+unicode(z)"
    }
}

fn set_slot_consumed(app: &AppHandle, paste_id: u64) {
    let manager = app.state::<UndoManager>();
    let now = now_ms();

    let mut slot_guard = manager.recent_slot.lock().unwrap();
    if let Some(slot) = slot_guard.as_mut() {
        if slot.paste_id == paste_id {
            slot.consume();
            info!(
                event_code = "undo_slot_consumed",
                paste_id = slot.paste_id,
                age_ms = now.saturating_sub(slot.created_at_ms),
                "Tracked undo slot consumed"
            );
        }
    }
}

pub fn mark_stop_transition_marker(app: &AppHandle) {
    let manager = app.state::<UndoManager>();
    let mut marker = manager.stop_transition_started_ms.lock().unwrap();
    *marker = Some(now_ms());
}

pub fn clear_stop_transition_marker(app: &AppHandle) {
    let manager = app.state::<UndoManager>();
    let mut marker = manager.stop_transition_started_ms.lock().unwrap();
    *marker = None;
}

fn has_active_stop_transition(app: &AppHandle) -> bool {
    let manager = app.state::<UndoManager>();
    let now = now_ms();
    let mut marker = manager.stop_transition_started_ms.lock().unwrap();

    match *marker {
        Some(started_ms) if now.saturating_sub(started_ms) <= STOP_TRANSITION_WINDOW_MS => true,
        Some(_) => {
            *marker = None;
            false
        }
        None => false,
    }
}

fn schedule_discoverability_hint(app: AppHandle) {
    info!(
        event_code = "undo_discoverability_hint_scheduled",
        delay_ms = DISCOVERABILITY_HINT_DELAY_MS,
        "Scheduled discoverability hint evaluation"
    );

    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(
            DISCOVERABILITY_HINT_DELAY_MS,
        ))
        .await;

        let evidence = load_evidence(&app);
        let shortcut_available = is_undo_shortcut_available(&app);
        if let Some(reason) = discoverability_skip_reason(&evidence, shortcut_available) {
            info!(
                event_code = "undo_discoverability_hint_skipped",
                reason = reason,
                successful_paste_count = evidence.successful_paste_count,
                "Skipped discoverability hint"
            );
            return;
        }

        let event = UndoUiEvent {
            kind: UndoUiKind::DiscoverabilityHint,
            code: "undo_discoverability_hint".to_string(),
            shortcut: Some(short_undo_binding(&app)),
        };

        info!(
            event_code = "undo_discoverability_hint_emitted",
            successful_paste_count = evidence.successful_paste_count,
            "Emitting discoverability hint"
        );

        emit_ui_event(&app, event);
    });
}

fn is_transcribe_source_action(source_action: &str) -> bool {
    matches!(source_action, "transcribe" | "transcribe_with_post_process")
}

pub fn reserve_stats_token(app: &AppHandle) -> u64 {
    let manager = app.state::<UndoManager>();
    manager.stats_token_counter.fetch_add(1, Ordering::SeqCst)
}

fn prune_stale_stats_contributions(
    pending_rollbacks: &HashSet<u64>,
    contributions: &mut HashMap<u64, TrackedStatsContribution>,
    now: u64,
    source: &str,
) {
    let before = contributions.len();
    contributions.retain(|token, tracked| {
        if pending_rollbacks.contains(token) {
            return true;
        }

        now.saturating_sub(tracked.inserted_at_ms) <= RECENT_PASTE_TTL_MS
    });

    let pruned_count = before.saturating_sub(contributions.len());
    if pruned_count > 0 {
        info!(
            event_code = "undo_stats_contribution_pruned",
            source = source,
            pruned_count = pruned_count,
            remaining_count = contributions.len(),
            "Pruned stale stats contributions"
        );
    }
}

fn apply_stats_rollback(
    app: &AppHandle,
    stats_token: u64,
    source_action: &str,
    contribution: StatsContribution,
) {
    let Some(history_manager_state) = app.try_state::<Arc<HistoryManager>>() else {
        warn!(
            event_code = "undo_stats_rollback_skipped",
            reason = "history_manager_unavailable",
            stats_token = stats_token,
            source_action = %source_action,
            "Stats rollback skipped: history manager unavailable"
        );
        return;
    };

    match history_manager_state.rollback_stats_contribution(&contribution) {
        Ok(_) => {
            info!(
                event_code = "undo_stats_rollback_applied",
                stats_token = stats_token,
                source_action = %source_action,
                word_count = contribution.word_count,
                effective_duration_ms = contribution.effective_duration_ms,
                filler_words_removed = contribution.filler_words_removed,
                date_key = %contribution.date_key,
                "Applied stats rollback for undone transcript"
            );
        }
        Err(error) => {
            warn!(
                event_code = "undo_stats_rollback_skipped",
                reason = "rollback_failed",
                stats_token = stats_token,
                source_action = %source_action,
                error = %error,
                "Stats rollback failed"
            );
        }
    }
}

fn schedule_pending_stats_rollback_expiry(app: AppHandle, stats_token: u64, source_action: String) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(RECENT_PASTE_TTL_MS)).await;

        let manager = app.state::<UndoManager>();
        let mut pending = manager.pending_stats_rollbacks.lock().unwrap();
        if pending.remove(&stats_token) {
            warn!(
                event_code = "undo_stats_rollback_skipped",
                reason = "contribution_missing_after_expiry",
                stats_token = stats_token,
                source_action = %source_action,
                "Stats rollback skipped after waiting for contribution"
            );
        }
    });
}

fn request_stats_rollback(app: &AppHandle, slot: &RecentPasteSlot) {
    {
        let manager = app.state::<UndoManager>();
        // Lock-order invariant: always acquire pending-rollbacks before contributions.
        let pending_rollbacks = manager.pending_stats_rollbacks.lock().unwrap();
        let mut contributions = manager.stats_contributions.lock().unwrap();
        prune_stale_stats_contributions(
            &pending_rollbacks,
            &mut contributions,
            now_ms(),
            "request_stats_rollback",
        );
    }

    if !is_transcribe_source_action(&slot.source_action) {
        info!(
            event_code = "undo_stats_rollback_skipped",
            reason = "non_transcribe_source",
            paste_id = slot.paste_id,
            source_action = %slot.source_action,
            "Stats rollback skipped for non-transcribe source action"
        );
        return;
    }

    let Some(stats_token) = slot.stats_token else {
        warn!(
            event_code = "undo_stats_rollback_skipped",
            reason = "no_token",
            paste_id = slot.paste_id,
            source_action = %slot.source_action,
            "Stats rollback skipped because transcribe slot had no token"
        );
        return;
    };

    info!(
        event_code = "undo_stats_rollback_requested",
        paste_id = slot.paste_id,
        stats_token = stats_token,
        source_action = %slot.source_action,
        "Stats rollback requested for undo slot"
    );

    let manager = app.state::<UndoManager>();
    // Lock-order invariant: always acquire pending-rollbacks before contributions.
    let mut pending_rollbacks = manager.pending_stats_rollbacks.lock().unwrap();
    let mut contributions = manager.stats_contributions.lock().unwrap();
    let contribution = contributions
        .remove(&stats_token)
        .map(|tracked| tracked.contribution);

    if let Some(contribution) = contribution {
        pending_rollbacks.remove(&stats_token);
        drop(contributions);
        drop(pending_rollbacks);
        apply_stats_rollback(app, stats_token, &slot.source_action, contribution);
        return;
    }

    let inserted = pending_rollbacks.insert(stats_token);
    drop(contributions);
    drop(pending_rollbacks);

    if inserted {
        info!(
            event_code = "undo_stats_rollback_deferred",
            paste_id = slot.paste_id,
            stats_token = stats_token,
            source_action = %slot.source_action,
            "Stats contribution not available yet; rollback deferred"
        );
        schedule_pending_stats_rollback_expiry(
            app.clone(),
            stats_token,
            slot.source_action.clone(),
        );
    }
}

pub fn register_stats_contribution(
    app: &AppHandle,
    stats_token: u64,
    source_action: &str,
    contribution: StatsContribution,
) {
    let manager = app.state::<UndoManager>();
    let now = now_ms();
    // Lock-order invariant: always acquire pending-rollbacks before contributions.
    let mut pending_rollbacks = manager.pending_stats_rollbacks.lock().unwrap();
    let mut contributions = manager.stats_contributions.lock().unwrap();
    prune_stale_stats_contributions(
        &pending_rollbacks,
        &mut contributions,
        now,
        "register_stats_contribution",
    );

    if pending_rollbacks.remove(&stats_token) {
        drop(contributions);
        drop(pending_rollbacks);
        apply_stats_rollback(app, stats_token, source_action, contribution);
        return;
    }

    contributions.insert(
        stats_token,
        TrackedStatsContribution {
            contribution,
            inserted_at_ms: now,
        },
    );
}

pub fn register_successful_paste(app: &AppHandle, capture: PasteCapture) {
    let manager = app.state::<UndoManager>();
    let now = now_ms();
    let paste_id = manager.paste_id_counter.fetch_add(1, Ordering::SeqCst);

    let slot = RecentPasteSlot {
        paste_id,
        source_action: capture.source_action.to_string(),
        stats_token: capture.stats_token,
        auto_refined: capture.auto_refined,
        pasted_text: capture.pasted_text,
        suggestion_text: capture.suggestion_text,
        created_at_ms: now,
        expires_at_ms: now.saturating_add(RECENT_PASTE_TTL_MS),
        consumed: false,
    };

    let previous_slot = {
        let mut slot_guard = manager.recent_slot.lock().unwrap();
        let previous = slot_guard
            .as_ref()
            .map(|prev| (prev.paste_id, prev.stats_token));
        *slot_guard = Some(slot.clone());
        previous
    };

    // Lock-order invariant: always acquire pending-rollbacks before contributions.
    let pending_rollbacks = manager.pending_stats_rollbacks.lock().unwrap();
    let mut contributions = manager.stats_contributions.lock().unwrap();
    prune_stale_stats_contributions(
        &pending_rollbacks,
        &mut contributions,
        now,
        "register_successful_paste",
    );

    if let Some((previous_paste_id, previous_stats_token)) = previous_slot {
        if let Some(previous_token) = previous_stats_token {
            if !pending_rollbacks.contains(&previous_token) {
                contributions.remove(&previous_token);
            }
        }

        info!(
            event_code = "undo_slot_overwritten",
            previous_paste_id = previous_paste_id,
            paste_id = slot.paste_id,
            "Tracked undo slot overwritten by newer paste"
        );
    }

    drop(contributions);
    drop(pending_rollbacks);

    info!(
        event_code = "undo_slot_created",
        paste_id = slot.paste_id,
        source_action = %slot.source_action,
        auto_refined = slot.auto_refined,
        "Tracked undo slot created"
    );

    let mut evidence = load_evidence(app);
    evidence.successful_paste_count = evidence.successful_paste_count.saturating_add(1);
    persist_evidence(app, &evidence);

    if !evidence.has_seen_undo_hint
        && !evidence.has_used_undo
        && evidence.successful_paste_count >= 2
    {
        if is_undo_shortcut_available(app) {
            schedule_discoverability_hint(app.clone());
        } else {
            info!(
                event_code = "undo_discoverability_hint_not_scheduled",
                reason = "undo_shortcut_unavailable",
                successful_paste_count = evidence.successful_paste_count,
                "Skipped discoverability hint scheduling"
            );
        }
    }
}

fn dispatch_undo_on_main_thread(app: &AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let result = (|| {
            let enigo_state = app_for_main
                .try_state::<EnigoState>()
                .ok_or_else(|| "Enigo state not initialized".to_string())?;

            if !enigo_state.is_available() {
                enigo_state.try_init();
            }

            let mut guard = enigo_state
                .0
                .lock()
                .map_err(|error| format!("Failed to lock Enigo: {error}"))?;
            let enigo = guard
                .as_mut()
                .ok_or_else(|| "Enigo unavailable".to_string())?;
            input::send_undo_cmd_z(enigo)
        })();

        let _ = sender.send(result);
    })
    .map_err(|error| format!("Failed to run undo dispatch on main thread: {error:?}"))?;

    receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|error| format!("Failed to receive undo dispatch result: {error}"))?
}

enum SlotStatus {
    Missing,
    Consumed(RecentPasteSlot),
    Expired(RecentPasteSlot),
    Valid(RecentPasteSlot),
}

fn slot_status(app: &AppHandle) -> SlotStatus {
    let manager = app.state::<UndoManager>();
    let now = now_ms();
    let mut slot_guard = manager.recent_slot.lock().unwrap();

    match slot_guard.as_ref() {
        None => SlotStatus::Missing,
        Some(slot) if slot.is_valid(now) => SlotStatus::Valid(slot.clone()),
        Some(slot) if slot.consumed => SlotStatus::Consumed(slot.clone()),
        Some(slot) if slot.is_expired(now) => {
            let expired_slot = slot.clone();
            info!(
                event_code = "undo_slot_expired",
                paste_id = expired_slot.paste_id,
                age_ms = now.saturating_sub(expired_slot.created_at_ms),
                "Tracked undo slot expired"
            );
            *slot_guard = None;
            SlotStatus::Expired(expired_slot)
        }
        Some(slot) => {
            warn!(
                event_code = "undo_slot_state_invariant_violation",
                paste_id = slot.paste_id,
                consumed = slot.consumed,
                created_at_ms = slot.created_at_ms,
                expires_at_ms = slot.expires_at_ms,
                now_ms = now,
                "Undo slot matched no known state branch; treating as missing"
            );
            debug_assert!(false, "Undo slot state invariant violated");
            *slot_guard = None;
            SlotStatus::Missing
        }
    }
}

fn feedback_event(code: &str) -> UndoUiEvent {
    UndoUiEvent {
        kind: UndoUiKind::Feedback,
        code: code.to_string(),
        shortcut: None,
    }
}

pub fn trigger_undo_last_transcript(app: &AppHandle) {
    let audio_manager = app.state::<std::sync::Arc<AudioRecordingManager>>();
    let transcription_manager = app.state::<std::sync::Arc<TranscriptionManager>>();

    let recording_active =
        audio_manager.get_active_binding_id().is_some() || audio_manager.is_recording();
    let transcribing_active = transcription_manager.is_any_session_active();
    let stop_transition_active = has_active_stop_transition(app);

    if recording_active || transcribing_active || stop_transition_active {
        info!(
            event_code = "undo_operation_cancel_requested",
            recording_active = recording_active,
            transcribing_active = transcribing_active,
            stop_transition_active = stop_transition_active,
            "Undo requested while operation active; triggering cancellation first"
        );

        if stop_transition_active && !recording_active && !transcribing_active {
            info!(
                event_code = "undo_stop_transition_cancel_requested",
                "Undo requested during stop transition window"
            );
        }

        utils::cancel_current_operation(app);

        info!(
            event_code = "undo_operation_cancel_completed",
            "Undo cancellation path completed"
        );

        info!(
            event_code = "undo_operation_cancel_short_circuit",
            "Undo skipped after cancellation to preserve legacy cancelling overlay UX"
        );

        return;
    }

    match slot_status(app) {
        SlotStatus::Missing => {
            info!(
                event_code = "undo_dispatch_skipped",
                reason = "missing_slot",
                "Undo skipped: no tracked slot"
            );
            emit_ui_event(app, feedback_event("undo_noop_empty"));
        }
        SlotStatus::Consumed(slot) => {
            info!(
                event_code = "undo_dispatch_skipped",
                reason = "consumed_slot",
                paste_id = slot.paste_id,
                "Undo skipped: slot already consumed"
            );
            emit_ui_event(app, feedback_event("undo_noop_empty"));
        }
        SlotStatus::Expired(slot) => {
            info!(
                event_code = "undo_dispatch_skipped",
                reason = "expired_slot",
                paste_id = slot.paste_id,
                "Undo skipped: slot expired"
            );
            emit_ui_event(app, feedback_event("undo_noop_expired"));
        }
        SlotStatus::Valid(slot) => {
            info!(
                event_code = "undo_dispatch_attempted",
                paste_id = slot.paste_id,
                source_action = %slot.source_action,
                auto_refined = slot.auto_refined,
                key_path = platform_undo_key_path(),
                "Dispatching platform undo for tracked slot"
            );

            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(
                    UNDO_MODIFIER_RELEASE_DELAY_MS,
                ))
                .await;

                if let Err(error) = dispatch_undo_on_main_thread(&app_clone) {
                    warn!(
                        event_code = "undo_dispatch_failed",
                        paste_id = slot.paste_id,
                        error = %error,
                        "Undo dispatch failed"
                    );
                    emit_ui_event(&app_clone, feedback_event("undo_failed"));
                    return;
                }

                set_slot_consumed(&app_clone, slot.paste_id);
                request_stats_rollback(&app_clone, &slot);

                let mut evidence = load_evidence(&app_clone);
                evidence.has_used_undo = true;
                persist_evidence(&app_clone, &evidence);

                emit_ui_event(&app_clone, feedback_event("undo_success"));
            });
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn undo_overlay_card_dismissed(app: AppHandle) {
    info!(
        event_code = "undo_overlay_interaction_mode_requested",
        mode = "dismissed",
        passthrough = true,
        "Undo overlay card dismissed"
    );
    overlay::set_overlay_input_mode_passthrough(&app, "undo_overlay_card_dismissed");
}

#[tauri::command]
#[specta::specta]
pub fn undo_overlay_card_presented(app: AppHandle) {
    info!(
        event_code = "undo_overlay_interaction_mode_requested",
        mode = "presented",
        passthrough = false,
        "Undo overlay card presented"
    );
    overlay::set_overlay_input_mode_undo(&app, "undo_overlay_card_presented");
}

#[tauri::command]
#[specta::specta]
pub fn undo_mark_discoverability_hint_seen(app: AppHandle) {
    let mut evidence = load_evidence(&app);
    if evidence.has_seen_undo_hint {
        info!(
            event_code = "undo_discoverability_hint_seen_already",
            "Discoverability hint seen flag already set"
        );
        return;
    }
    evidence.has_seen_undo_hint = true;
    persist_evidence(&app, &evidence);
    info!(
        event_code = "undo_discoverability_hint_seen_marked",
        successful_paste_count = evidence.successful_paste_count,
        has_used_undo = evidence.has_used_undo,
        "Persisted discoverability hint seen flag"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn sample_slot(created_at_ms: u64) -> RecentPasteSlot {
        RecentPasteSlot {
            paste_id: 1,
            source_action: "transcribe".to_string(),
            stats_token: None,
            auto_refined: false,
            pasted_text: "hello".to_string(),
            suggestion_text: "hello".to_string(),
            created_at_ms,
            expires_at_ms: created_at_ms + RECENT_PASTE_TTL_MS,
            consumed: false,
        }
    }

    fn sample_contribution(word_count: i64) -> StatsContribution {
        StatsContribution {
            word_count,
            effective_duration_ms: 1_000,
            filler_words_removed: 0,
            date_added_to_streak_list: false,
            date_key: "2026-02-17".to_string(),
        }
    }

    #[test]
    fn recent_paste_slot_ttl_validity() {
        let slot = sample_slot(1_000);
        assert!(slot.is_valid(1_000));
        assert!(slot.is_valid(1_000 + RECENT_PASTE_TTL_MS));
        assert!(!slot.is_expired(1_000 + RECENT_PASTE_TTL_MS));
        assert!(slot.is_expired(1_000 + RECENT_PASTE_TTL_MS + 1));
        assert!(!slot.is_valid(1_000 + RECENT_PASTE_TTL_MS + 1));
    }

    #[test]
    fn recent_paste_slot_single_use_consume() {
        let mut slot = sample_slot(1_000);
        assert!(!slot.consumed);
        assert!(slot.is_valid(1_001));
        slot.consume();
        assert!(slot.consumed);
        assert!(!slot.is_valid(1_001));
        slot.consume();
        assert!(slot.consumed);
    }

    #[test]
    fn recent_paste_slot_overwrite_semantics() {
        let mut slot = Some(sample_slot(1_000));
        assert_eq!(slot.as_ref().map(|s| s.paste_id), Some(1));

        let mut second = sample_slot(2_000);
        second.paste_id = 2;
        second.pasted_text = "second".to_string();
        let previous = slot.replace(second.clone());

        assert_eq!(previous.as_ref().map(|s| s.paste_id), Some(1));
        assert_eq!(slot.as_ref().map(|s| s.paste_id), Some(2));
        assert_eq!(
            slot.as_ref().map(|s| s.pasted_text.as_str()),
            Some("second")
        );
    }

    #[test]
    fn stats_rollback_source_scope_matches_transcribe_actions() {
        assert!(is_transcribe_source_action("transcribe"));
        assert!(is_transcribe_source_action("transcribe_with_post_process"));
        assert!(!is_transcribe_source_action("paste_last_transcript"));
        assert!(!is_transcribe_source_action("refine_last_transcript"));
    }

    #[test]
    fn prune_stale_stats_contributions_removes_expired_non_pending_entries() {
        let now = RECENT_PASTE_TTL_MS + 10_000;
        let pending = HashSet::new();
        let mut contributions = HashMap::new();

        contributions.insert(
            1,
            TrackedStatsContribution {
                contribution: sample_contribution(10),
                inserted_at_ms: 0,
            },
        );
        contributions.insert(
            2,
            TrackedStatsContribution {
                contribution: sample_contribution(20),
                inserted_at_ms: now.saturating_sub(1_000),
            },
        );

        prune_stale_stats_contributions(
            &pending,
            &mut contributions,
            now,
            "prune_stale_stats_contributions_test",
        );

        assert!(!contributions.contains_key(&1));
        assert!(contributions.contains_key(&2));
    }

    #[test]
    fn prune_stale_stats_contributions_keeps_expired_pending_entries() {
        let now = RECENT_PASTE_TTL_MS + 10_000;
        let mut pending = HashSet::new();
        let mut contributions = HashMap::new();

        pending.insert(7);
        contributions.insert(
            7,
            TrackedStatsContribution {
                contribution: sample_contribution(7),
                inserted_at_ms: 0,
            },
        );

        prune_stale_stats_contributions(
            &pending,
            &mut contributions,
            now,
            "prune_stale_stats_contributions_test",
        );

        assert!(contributions.contains_key(&7));
    }

    #[test]
    fn discoverability_skip_reason_requires_shortcut_availability() {
        let evidence = UndoNudgeEvidence {
            has_seen_undo_hint: false,
            successful_paste_count: 2,
            has_used_undo: false,
        };

        assert_eq!(
            discoverability_skip_reason(&evidence, false),
            Some("undo_shortcut_unavailable")
        );
        assert_eq!(discoverability_skip_reason(&evidence, true), None);
    }

    #[test]
    fn discoverability_skip_reason_prioritizes_evidence_flags() {
        let evidence = UndoNudgeEvidence {
            has_seen_undo_hint: true,
            successful_paste_count: 99,
            has_used_undo: false,
        };
        assert_eq!(
            discoverability_skip_reason(&evidence, false),
            Some("has_seen")
        );
    }
}
