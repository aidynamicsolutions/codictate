#[cfg(target_os = "macos")]
mod accessibility;
mod actions;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple_intelligence;
mod audio_device_info;
mod audio_feedback;
pub mod audio_toolkit;
pub mod cli;
mod clipboard;
mod commands;
#[cfg(target_os = "macos")]
mod fn_key_monitor;
mod helpers;
mod i18n;
mod input;
mod llm_client;
mod managers;
mod menu;
mod notification;
mod overlay;
mod permissions;
mod settings;
mod sentry_observability;
mod shortcut;
mod signal_handle;
mod smart_insertion;
mod tracing_config;
mod transcription_coordinator;
mod tray;
mod tray_i18n;
mod undo;
mod user_profile;
mod utils;

pub use cli::CliArgs;
use once_cell::sync::Lazy;
use regex::Regex;
use sentry::protocol::{Event as SentryEvent, Map as SentryMap, Stacktrace};
use serde_json::Value as JsonValue;
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{collect_commands, Builder};

use env_filter::Builder as EnvFilterBuilder;
use managers::audio::AudioRecordingManager;
use managers::correction::CorrectionManager;
use managers::history::HistoryManager;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use managers::mlx::MlxModelManager;
use managers::model::ModelManager;
use managers::transcription::TranscriptionManager;
#[cfg(unix)]
use signal_hook::consts::{SIGUSR1, SIGUSR2};
#[cfg(unix)]
use signal_hook::iterator::Signals;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::image::Image;
pub use transcription_coordinator::TranscriptionCoordinator;

use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::settings::get_settings;
use crate::sentry_observability::initialize_sentry_identity_scope;

#[derive(Default)]
struct ShortcutToggleStates {
    // Map: shortcut_binding_id -> is_active
    active_toggles: HashMap<String, bool>,
}

type ManagedToggleState = Mutex<ShortcutToggleStates>;

/// Global state to override paste method during onboarding.
/// When true, forces Direct paste method to work around WebView not receiving
/// CGEvent-simulated Cmd+V keystrokes from the same process.
pub type OnboardingPasteOverride = Mutex<bool>;

// Global atomic to store the file log level filter
// We use u8 to store the log::LevelFilter as a number
pub static FILE_LOG_LEVEL: AtomicU8 = AtomicU8::new(log::LevelFilter::Debug as u8);

const SENTRY_DSN_ENV_VAR: &str = "SENTRY_DSN";
const SENTRY_ENVIRONMENT_ENV_VAR: &str = "SENTRY_ENVIRONMENT";
const SENTRY_RELEASE_ENV_VAR: &str = "SENTRY_RELEASE";
const HANDY_DISABLE_SENTRY_ENV_VAR: &str = "HANDY_DISABLE_SENTRY";
const HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START_ENV_VAR: &str =
    "HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START";
const BUILD_TIME_SENTRY_DSN: Option<&str> = option_env!("SENTRY_DSN");

#[derive(Clone, Copy)]
enum SentryDsnSource {
    RuntimeEnv,
    BuildTimeEmbedded,
}

static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b")
        .expect("email scrubbing pattern should compile")
});
static IPV4_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b",
    )
    .expect("ipv4 scrubbing pattern should compile")
});
static UNIX_USER_PATH_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)/(users|home)/[^/\s]+").expect("unix path scrubbing pattern should compile")
});
static WINDOWS_USER_PATH_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[a-z]:\\users\\[^\\\s]+")
        .expect("windows path scrubbing pattern should compile")
});
static SENSITIVE_ASSIGNMENT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(api[_-]?key|access[_-]?token|refresh[_-]?token|token|password|secret|authorization|cookie)\b\s*[:=]\s*([^\s,;]+)",
    )
    .expect("sensitive assignment scrubbing pattern should compile")
});


fn build_console_filter() -> env_filter::Filter {
    let mut builder = EnvFilterBuilder::new();

    match std::env::var("RUST_LOG") {
        Ok(spec) if !spec.trim().is_empty() => {
            if let Err(err) = builder.try_parse(&spec) {
                log::warn!(
                    "Ignoring invalid RUST_LOG value '{}': {}. Falling back to info-level console logging",
                    spec,
                    err
                );
                builder.filter_level(log::LevelFilter::Info);
            }
        }
        _ => {
            builder.filter_level(log::LevelFilter::Info);
        }
    }

    builder.build()
}

fn env_var_is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn normalize_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_sentry_dsn() -> Option<(String, SentryDsnSource)> {
    if let Some(runtime_dsn) = std::env::var(SENTRY_DSN_ENV_VAR)
        .ok()
        .and_then(|value| normalize_non_empty(&value))
    {
        return Some((runtime_dsn, SentryDsnSource::RuntimeEnv));
    }

    BUILD_TIME_SENTRY_DSN
        .and_then(normalize_non_empty)
        .map(|dsn| (dsn, SentryDsnSource::BuildTimeEmbedded))
}

fn resolve_sentry_environment() -> String {
    std::env::var(SENTRY_ENVIRONMENT_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "development".to_string())
}

fn resolve_sentry_release() -> String {
    std::env::var(SENTRY_RELEASE_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("codictate@{}", env!("CARGO_PKG_VERSION")))
}

fn sanitize_text(input: &str) -> String {
    let step1 = EMAIL_PATTERN.replace_all(input, "[redacted-email]").into_owned();
    let step2 = sanitize_ipv4_candidates(&step1);
    let step3 = UNIX_USER_PATH_PATTERN
        .replace_all(&step2, "/$1/[redacted-user]")
        .into_owned();
    let step4 = WINDOWS_USER_PATH_PATTERN
        .replace_all(&step3, "C:\\Users\\[redacted-user]")
        .into_owned();

    SENSITIVE_ASSIGNMENT_PATTERN
        .replace_all(&step4, "$1=[redacted]")
        .into_owned()
}

fn sanitize_ipv4_candidates(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    let mut last_index = 0;

    for matched in IPV4_PATTERN.find_iter(input) {
        sanitized.push_str(&input[last_index..matched.start()]);

        // Do not scrub release-like tokens such as `codictate@0.7.6.1`.
        // This guard is intentionally narrow to avoid preserving true IPs in
        // other key-value contexts.
        let previous_char = input[..matched.start()].chars().next_back();
        if previous_char == Some('@') {
            sanitized.push_str(matched.as_str());
        } else {
            sanitized.push_str("[redacted-ip]");
        }

        last_index = matched.end();
    }

    sanitized.push_str(&input[last_index..]);
    sanitized
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    let has_token_marker = normalized == "token"
        || normalized.starts_with("token_")
        || normalized.ends_with("_token")
        || normalized.contains("_token_")
        || normalized.starts_with("token-")
        || normalized.ends_with("-token")
        || normalized.contains("-token-")
        || normalized.starts_with("token.")
        || normalized.ends_with(".token")
        || normalized.contains(".token.")
        // Catch common flattened camelCase variants such as `accessToken`.
        || normalized.ends_with("token");

    normalized.contains("password")
        || has_token_marker
        || normalized.contains("secret")
        || normalized.contains("authorization")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("cookie")
        || normalized.contains("session")
        || normalized.contains("credential")
}

fn scrub_optional_string(value: &mut Option<String>) {
    if let Some(text) = value {
        *text = sanitize_text(text);
    }
}

fn scrub_json_value(value: &mut JsonValue) {
    match value {
        JsonValue::String(text) => {
            *text = sanitize_text(text);
        }
        JsonValue::Array(items) => {
            for item in items {
                scrub_json_value(item);
            }
        }
        JsonValue::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if let Some(entry) = map.get_mut(&key) {
                    if is_sensitive_key(&key) {
                        *entry = JsonValue::String("[redacted]".to_string());
                    } else {
                        scrub_json_value(entry);
                    }
                }
            }
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn scrub_string_map(map: &mut SentryMap<String, String>) {
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if let Some(value) = map.get_mut(&key) {
            if is_sensitive_key(&key) {
                *value = "[redacted]".to_string();
            } else {
                *value = sanitize_text(value);
            }
        }
    }
}

fn scrub_stacktrace(stacktrace: &mut Stacktrace) {
    for frame in &mut stacktrace.frames {
        scrub_optional_string(&mut frame.abs_path);
        scrub_optional_string(&mut frame.filename);
        scrub_optional_string(&mut frame.module);
        scrub_optional_string(&mut frame.function);
        scrub_optional_string(&mut frame.symbol);
        scrub_optional_string(&mut frame.package);
        scrub_optional_string(&mut frame.context_line);

        for line in &mut frame.pre_context {
            *line = sanitize_text(line);
        }
        for line in &mut frame.post_context {
            *line = sanitize_text(line);
        }

        let keys: Vec<String> = frame.vars.keys().cloned().collect();
        for key in keys {
            if let Some(value) = frame.vars.get_mut(&key) {
                if is_sensitive_key(&key) {
                    *value = JsonValue::String("[redacted]".to_string());
                } else {
                    scrub_json_value(value);
                }
            }
        }
    }
}

fn scrub_sentry_event(mut event: SentryEvent<'static>) -> SentryEvent<'static> {
    scrub_optional_string(&mut event.culprit);
    scrub_optional_string(&mut event.transaction);
    scrub_optional_string(&mut event.message);
    scrub_optional_string(&mut event.logger);

    if let Some(logentry) = event.logentry.as_mut() {
        logentry.message = sanitize_text(&logentry.message);
        for param in &mut logentry.params {
            scrub_json_value(param);
        }
    }

    if let Some(user) = event.user.as_mut() {
        if user.email.is_some() {
            user.email = Some("[redacted-email]".to_string());
        }
        user.ip_address = None;
        if let Some(user_id) = user.id.as_deref() {
            if !user_id.starts_with("anon:") {
                // Preserve pseudonymous `anon:` IDs, and redact all other identifiers.
                user.id = Some("[redacted-user-id]".to_string());
            }
        }
        scrub_optional_string(&mut user.username);

        let keys: Vec<String> = user.other.keys().cloned().collect();
        for key in keys {
            if let Some(value) = user.other.get_mut(&key) {
                if is_sensitive_key(&key) {
                    *value = JsonValue::String("[redacted]".to_string());
                } else {
                    scrub_json_value(value);
                }
            }
        }
    }

    if let Some(request) = event.request.as_mut() {
        request.url = None;
        scrub_optional_string(&mut request.data);
        scrub_optional_string(&mut request.query_string);
        request.cookies = None;
        scrub_string_map(&mut request.headers);
        scrub_string_map(&mut request.env);
    }

    for exception in &mut event.exception.values {
        exception.ty = sanitize_text(&exception.ty);
        scrub_optional_string(&mut exception.value);
        scrub_optional_string(&mut exception.module);
        if let Some(stacktrace) = exception.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
        if let Some(raw_stacktrace) = exception.raw_stacktrace.as_mut() {
            scrub_stacktrace(raw_stacktrace);
        }
    }

    if let Some(stacktrace) = event.stacktrace.as_mut() {
        scrub_stacktrace(stacktrace);
    }

    for thread in &mut event.threads.values {
        scrub_optional_string(&mut thread.name);
        if let Some(stacktrace) = thread.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
        if let Some(raw_stacktrace) = thread.raw_stacktrace.as_mut() {
            scrub_stacktrace(raw_stacktrace);
        }
    }

    for breadcrumb in &mut event.breadcrumbs.values {
        scrub_optional_string(&mut breadcrumb.message);
        scrub_optional_string(&mut breadcrumb.category);

        let keys: Vec<String> = breadcrumb.data.keys().cloned().collect();
        for key in keys {
            if let Some(value) = breadcrumb.data.get_mut(&key) {
                if is_sensitive_key(&key) {
                    *value = JsonValue::String("[redacted]".to_string());
                } else {
                    scrub_json_value(value);
                }
            }
        }
    }

    scrub_string_map(&mut event.tags);

    for value in event.modules.values_mut() {
        *value = sanitize_text(value);
    }

    let keys: Vec<String> = event.extra.keys().cloned().collect();
    for key in keys {
        if let Some(value) = event.extra.get_mut(&key) {
            if is_sensitive_key(&key) {
                *value = JsonValue::String("[redacted]".to_string());
            } else {
                scrub_json_value(value);
            }
        }
    }

    if event.release.is_none() {
        event.release = Some(Cow::Owned(resolve_sentry_release()));
    }
    if event.environment.is_none() {
        event.environment = Some(Cow::Owned(resolve_sentry_environment()));
    }

    event
}

fn initialize_sentry() -> (Option<sentry::ClientInitGuard>, String) {
    if let Ok(disable_value) = std::env::var(HANDY_DISABLE_SENTRY_ENV_VAR) {
        if env_var_is_truthy(&disable_value) {
            return (
                None,
                format!(
                    "Sentry disabled: {} is set to '{}'",
                    HANDY_DISABLE_SENTRY_ENV_VAR, disable_value
                ),
            );
        }
    }

    let Some((dsn, dsn_source)) = resolve_sentry_dsn() else {
        return (
            None,
            format!(
                "Sentry disabled: {} is missing or empty and no build-time fallback is embedded",
                SENTRY_DSN_ENV_VAR
            ),
        );
    };

    let parsed_dsn = match dsn.parse::<sentry::types::Dsn>() {
        Ok(value) => value,
        Err(_) => {
            return (
                None,
                format!("Sentry disabled: {} is invalid", SENTRY_DSN_ENV_VAR),
            );
        }
    };

    let release = resolve_sentry_release();
    let environment = resolve_sentry_environment();

    let guard = sentry::init((
        parsed_dsn,
        sentry::ClientOptions {
            release: Some(Cow::Owned(release.clone())),
            environment: Some(Cow::Owned(environment.clone())),
            send_default_pii: false,
            before_send: Some(Arc::new(|event| Some(scrub_sentry_event(event)))),
            ..Default::default()
        },
    ));

    (
        Some(guard),
        format!(
            "Sentry enabled (release='{}', environment='{}', dsn_source='{}')",
            release,
            environment,
            match dsn_source {
                SentryDsnSource::RuntimeEnv => "runtime_env",
                SentryDsnSource::BuildTimeEmbedded => "build_time_embedded",
            }
        ),
    )
}

fn trigger_sentry_privacy_redaction_smoke_if_requested() {
    let Ok(value) = std::env::var(HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START_ENV_VAR) else {
        return;
    };

    if !env_var_is_truthy(&value) {
        return;
    }

    let has_active_sentry_client = sentry::Hub::with_active(|hub| hub.client().is_some());
    if !has_active_sentry_client {
        tracing::warn!(
            "Skipped privacy redaction smoke event because no active Sentry client is bound (check {} / {})",
            SENTRY_DSN_ENV_VAR,
            HANDY_DISABLE_SENTRY_ENV_VAR
        );
        return;
    }

    let event_id = sentry::with_scope(
        |scope| {
            scope.set_level(Some(sentry::Level::Error));
            scope.set_tag("smoke_test", "true");
            scope.set_tag("component", "backend");
            scope.set_tag("operation", "privacy_redaction_smoke");
            scope.set_extra("test_email", JsonValue::String("dev@example.com".to_string()));
            scope.set_extra(
                "test_path",
                JsonValue::String("/Users/alice/private/file.txt".to_string()),
            );
            scope.set_extra("test_ip", JsonValue::String("192.168.0.42".to_string()));
            scope.set_extra("api_key", JsonValue::String("abc123".to_string()));
            scope.set_extra("test_cookie", JsonValue::String("session=abcdef12345".to_string()));
        },
        || {
            sentry::capture_message(
                "privacy scrub smoke event: dev@example.com /Users/alice/private/file.txt 192.168.0.42 api_key=abc123",
                sentry::Level::Error,
            )
        },
    );

    tracing::warn!(
        "Triggered privacy redaction smoke event (event_id={}, env_var={})",
        event_id,
        HANDY_SENTRY_TEST_PRIVACY_REDACTION_ON_START_ENV_VAR
    );
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        // First, ensure the window is visible
        if let Err(e) = main_window.show() {
            tracing::error!("Failed to show window: {}", e);
        }
        // Then, bring it to the front and give it focus
        if let Err(e) = main_window.set_focus() {
            tracing::error!("Failed to focus window: {}", e);
        }
        // Optional: On macOS, ensure the app becomes active if it was an accessory
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = app.set_activation_policy(tauri::ActivationPolicy::Regular) {
                tracing::error!("Failed to set activation policy to Regular: {}", e);
            }
        }
    } else {
        tracing::error!("Main window not found.");
    }
}

fn initialize_core_logic(app_handle: &AppHandle) {
    // Create the recording overlay window immediately (hidden by default)
    // We do this FIRST so the webview (React app) can start loading its resources
    // in parallel with the backend initialization below. This significantly reduces
    // the "overlay ready" latency on first launch.
    utils::create_recording_overlay(app_handle);

    // Initialize the i18n system
    i18n::init(app_handle);

    // Initialize the input state (Enigo singleton for keyboard/mouse simulation)
    // This is lazy-initialized - if accessibility permissions are not granted,
    // Enigo will be None until permissions are granted and try_init() is called
    let enigo_state = input::EnigoState::new();
    app_handle.manage(enigo_state);

    // Initialize the managers
    let recording_manager = Arc::new(
        AudioRecordingManager::new(app_handle).expect("Failed to initialize recording manager"),
    );
    let model_manager =
        Arc::new(ModelManager::new(app_handle).expect("Failed to initialize model manager"));
    let transcription_manager = Arc::new(
        TranscriptionManager::new(app_handle, model_manager.clone())
            .expect("Failed to initialize transcription manager"),
    );
    let history_manager =
        Arc::new(HistoryManager::new(app_handle).expect("Failed to initialize history manager"));

    // Initialize MLX model manager for Apple Silicon Macs
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let mlx_manager =
        Arc::new(MlxModelManager::new(app_handle).expect("Failed to initialize MLX model manager"));

    // Add managers to Tauri's managed state
    app_handle.manage(recording_manager.clone());
    app_handle.manage(model_manager.clone());
    app_handle.manage(transcription_manager.clone());
    app_handle.manage(history_manager.clone());
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    app_handle.manage(mlx_manager.clone());

    // Initialize CorrectionManager for AI voice correction
    let correction_manager = Arc::new(CorrectionManager::new(app_handle.clone()));
    app_handle.manage(correction_manager);

    // Pre-warm Bluetooth microphone if selected
    // (triggers A2DP→HFP switch if needed)
    recording_manager.prewarm_bluetooth_mic();

    // Warm up the recorder (loads VAD model) without opening the mic
    // This removes the ~700ms delay on first record
    recording_manager.warmup_recorder();

    // Start background loading of the transcription model
    // This removes the ~1.5s delay on first transcription
    transcription_manager.initiate_model_load();

    // Bootstrap global shortcuts during backend startup so hidden/background
    // launches can use one-shot actions (e.g. undo_last_transcript) without
    // waiting for the main UI to mount.
    //
    // On macOS this may defer if accessibility permission is not yet granted;
    // frontend recovery paths retry initialization after permission transitions.
    let _ = commands::initialize_shortcuts_with_source(app_handle, "backend_startup");

    // Initialize Fn key monitor on macOS for transcription via Fn key
    #[cfg(target_os = "macos")]
    {
        let app_clone = app_handle.clone();
        // Start Fn key monitor with transcription enabled
        // The monitor itself now checks settings dynamically to decide if it should act
        std::thread::spawn(move || {
            let _ = fn_key_monitor::start_fn_key_monitor(app_clone, true);
        });
    }

    #[cfg(unix)]
    let signals = Signals::new(&[SIGUSR1, SIGUSR2]).unwrap();
    // Set up signal handlers for toggling transcription
    #[cfg(unix)]
    signal_handle::setup_signal_handler(app_handle.clone(), signals);

    // Apply macOS Accessory policy if starting hidden
    #[cfg(target_os = "macos")]
    {
        let settings = settings::get_settings(app_handle);
        if settings.start_hidden {
            let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
    // Get the current theme to set the appropriate initial icon
    let initial_theme = tray::get_current_theme(app_handle);

    // Choose the appropriate initial icon based on theme
    let initial_icon_path = tray::get_icon_path(initial_theme, tray::TrayIconState::Idle);

    let tray = TrayIconBuilder::new()
        .icon(
            Image::from_path(
                app_handle
                    .path()
                    .resolve(initial_icon_path, tauri::path::BaseDirectory::Resource)
                    .unwrap(),
            )
            .unwrap(),
        )
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                show_main_window(app);
            }
            "check_updates" => {
                let settings = settings::get_settings(app);
                if settings.update_checks_enabled {
                    show_main_window(app);
                    let _ = app.emit("check-for-updates", ());
                }
            }
            "unload_model" => {
                let transcription_manager = app.state::<Arc<TranscriptionManager>>();
                if !transcription_manager.is_model_loaded() {
                    tracing::warn!("No model is currently loaded.");
                    return;
                }
                match transcription_manager.unload_model() {
                    Ok(()) => tracing::info!("Model unloaded via tray."),
                    Err(e) => tracing::error!("Failed to unload model via tray: {}", e),
                }
            }
            "cancel" => {
                use crate::utils::cancel_current_operation;

                // Use centralized cancellation that handles all operations
                cancel_current_operation(app);
            }
            "copy_last_transcript" => {
                tray::copy_last_transcript(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app_handle)
        .unwrap();
    app_handle.manage(tray);

    // Initialize tray menu with idle state (spawn async since Idle needs history check)
    let app_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        utils::update_tray_menu_async(&app_clone, &utils::TrayIconState::Idle, None).await;
    });

    // Apply show_tray_icon setting
    let settings = settings::get_settings(app_handle);
    if !settings.show_tray_icon {
        tray::set_tray_visibility(app_handle, false);
    }

    // Refresh tray menu when model state changes
    let app_handle_for_listener = app_handle.clone();
    app_handle.listen("model-state-changed", move |_| {
        tray::update_tray_menu_sync(&app_handle_for_listener, &tray::TrayIconState::Idle, None);
    });

    // Get the autostart manager and configure based on user setting
    let autostart_manager = app_handle.autolaunch();
    let settings = settings::get_settings(app_handle);

    if settings.autostart_enabled {
        // Enable autostart if user has opted in
        let _ = autostart_manager.enable();
    } else {
        // Disable autostart if user has opted out
        let _ = autostart_manager.disable();
    }

    // Listen for "overlay-ready" event from the frontend
    // This signals that the React component has registered its event listeners
    // and is ready to receive show-overlay/hide-overlay events
    let app_for_overlay_ready = app_handle.clone();
    app_handle.listen("overlay-ready", move |_event| {
        // Pass the app handle so mark_overlay_ready can re-emit state if needed
        overlay::mark_overlay_ready(&app_for_overlay_ready);
        tracing::debug!("Received overlay-ready event from frontend");
    });
}

#[tauri::command]
#[specta::specta]
fn trigger_update_check(app: AppHandle) -> Result<(), String> {
    let settings = settings::get_settings(&app);
    if !settings.update_checks_enabled {
        return Ok(());
    }
    app.emit("check-for-updates", ())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(cli_args: CliArgs) {
    // Parse console logging directives from RUST_LOG, falling back to info-level logging
    // when the variable is unset
    let _console_filter = build_console_filter();

    // On Apple Silicon macOS, include MLX commands
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let specta_builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        shortcut::change_binding,
        shortcut::reset_binding,
        shortcut::reset_bindings,
        shortcut::change_audio_feedback_setting,
        shortcut::change_audio_feedback_volume_setting,
        shortcut::change_sound_theme_setting,
        shortcut::change_start_hidden_setting,
        shortcut::change_autostart_setting,
        shortcut::change_translate_to_english_setting,
        shortcut::change_selected_language_setting,
        shortcut::change_overlay_position_setting,
        shortcut::change_debug_mode_setting,
        shortcut::change_word_correction_threshold_setting,
        shortcut::change_paste_method_setting,
        shortcut::get_available_typing_tools,
        shortcut::change_typing_tool_setting,
        shortcut::change_external_script_path_setting,
        shortcut::change_clipboard_handling_setting,
        shortcut::change_auto_submit_setting,
        shortcut::change_auto_submit_key_setting,
        shortcut::change_post_process_enabled_setting,
        shortcut::change_post_process_base_url_setting,
        shortcut::change_post_process_api_key_setting,
        shortcut::change_post_process_model_setting,
        shortcut::set_post_process_provider,
        shortcut::fetch_post_process_models,
        shortcut::add_post_process_prompt,
        shortcut::update_post_process_prompt,
        shortcut::delete_post_process_prompt,
        shortcut::set_post_process_selected_prompt,
        shortcut::update_custom_words,
        shortcut::suspend_binding,
        shortcut::resume_binding,
        shortcut::change_mute_while_recording_setting,
        shortcut::change_append_trailing_space_setting,
        shortcut::change_paste_last_use_smart_insertion_setting,
        shortcut::change_filler_word_filter_setting,
        shortcut::change_hallucination_filter_setting,
        shortcut::change_app_language_setting,
        shortcut::change_update_checks_setting,
        shortcut::change_show_tray_icon_setting,
        shortcut::change_show_unload_model_in_tray_setting,
        user_profile::get_user_profile_command,
        user_profile::update_user_profile_setting,
        trigger_update_check,
        commands::cancel_operation,
        commands::get_app_dir_path,
        commands::get_app_settings,
        commands::get_default_settings,
        commands::reset_app_settings,
        commands::get_log_dir_path,
        commands::set_log_level,
        commands::open_recordings_folder,
        commands::open_log_dir,
        commands::open_app_data_dir,
        commands::check_apple_intelligence_available,
        commands::log_from_frontend,
        commands::set_onboarding_paste_override,
        commands::initialize_enigo,
        commands::initialize_shortcuts,
        commands::models::get_available_models,
        commands::models::get_model_info,
        commands::models::download_model,
        commands::models::delete_model,
        commands::models::cancel_download,
        commands::models::set_active_model,
        commands::models::get_current_model,
        commands::models::get_transcription_model_status,
        commands::models::is_model_loading,
        commands::models::has_any_models_available,
        commands::models::has_any_models_or_downloads,
        commands::models::get_recommended_first_model,
        commands::audio::update_microphone_mode,
        commands::audio::get_microphone_mode,
        commands::audio::get_available_microphones,
        commands::audio::set_selected_microphone,
        commands::audio::get_selected_microphone,
        commands::audio::get_available_output_devices,
        commands::audio::set_selected_output_device,
        commands::audio::get_selected_output_device,
        commands::audio::play_test_sound,
        commands::audio::check_custom_sounds,
        commands::audio::set_clamshell_microphone,
        commands::audio::get_clamshell_microphone,
        commands::audio::is_recording,
        commands::audio::start_mic_preview,
        commands::audio::stop_mic_preview,
        commands::transcription::set_model_unload_timeout,
        commands::transcription::get_model_load_status,
        commands::transcription::unload_model_manually,
        commands::history::get_history_entries,
        commands::history::toggle_history_entry_saved,
        commands::history::get_audio_file_path,
        commands::history::delete_history_entry,
        commands::history::update_history_limit,
        commands::history::update_recording_retention_period,
        commands::history::get_home_stats,
        commands::history::clear_all_history,
        commands::history::get_history_storage_usage,
        commands::history::prune_history,
        helpers::clamshell::is_laptop,
        // MLX commands
        commands::mlx::mlx_list_models,
        commands::mlx::mlx_get_model_status,
        commands::mlx::mlx_download_model,
        commands::mlx::mlx_cancel_download,
        commands::mlx::mlx_retry_download,
        commands::mlx::mlx_delete_model,
        commands::mlx::mlx_process_text,
        commands::mlx::mlx_is_busy,
        commands::mlx::mlx_unload_model,
        commands::mlx::mlx_switch_model,
        commands::mlx::mlx_open_models_dir,
        // macOS Fn key monitor commands
        fn_key_monitor::start_fn_key_monitor,
        fn_key_monitor::stop_fn_key_monitor,
        // Permission commands
        permissions::open_accessibility_settings,
        permissions::open_microphone_settings,
        commands::window::show_main_window,
        commands::menu::set_update_menu_text,
        commands::correction::accept_correction,
        commands::correction::dismiss_correction,
        undo::undo_overlay_card_dismissed,
        undo::undo_overlay_card_presented,
        undo::undo_mark_discoverability_hint_seen,
        overlay::overlay_update_interaction_regions,
    ]);

    // On other platforms, exclude MLX commands
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    let specta_builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        shortcut::change_binding,
        shortcut::reset_binding,
        shortcut::reset_bindings,
        shortcut::change_audio_feedback_setting,
        shortcut::change_audio_feedback_volume_setting,
        shortcut::change_sound_theme_setting,
        shortcut::change_start_hidden_setting,
        shortcut::change_autostart_setting,
        shortcut::change_translate_to_english_setting,
        shortcut::change_selected_language_setting,
        shortcut::change_overlay_position_setting,
        shortcut::change_debug_mode_setting,
        shortcut::change_word_correction_threshold_setting,
        shortcut::change_paste_method_setting,
        shortcut::change_clipboard_handling_setting,
        shortcut::change_post_process_enabled_setting,
        shortcut::change_post_process_base_url_setting,
        shortcut::change_post_process_api_key_setting,
        shortcut::change_post_process_model_setting,
        shortcut::set_post_process_provider,
        shortcut::fetch_post_process_models,
        shortcut::add_post_process_prompt,
        shortcut::update_post_process_prompt,
        shortcut::delete_post_process_prompt,
        shortcut::set_post_process_selected_prompt,
        shortcut::update_custom_words,
        shortcut::suspend_binding,
        shortcut::resume_binding,
        shortcut::change_mute_while_recording_setting,
        shortcut::change_append_trailing_space_setting,
        shortcut::change_paste_last_use_smart_insertion_setting,
        shortcut::change_filler_word_filter_setting,
        shortcut::change_hallucination_filter_setting,
        shortcut::change_app_language_setting,
        shortcut::change_update_checks_setting,
        shortcut::change_show_tray_icon_setting,
        shortcut::change_show_unload_model_in_tray_setting,
        shortcut::change_auto_submit_setting,
        shortcut::change_auto_submit_key_setting,
        user_profile::get_user_profile_command,
        user_profile::update_user_profile_setting,
        trigger_update_check,
        commands::cancel_operation,
        commands::get_app_dir_path,
        commands::get_app_settings,
        commands::get_default_settings,
        commands::reset_app_settings,
        commands::get_log_dir_path,
        commands::set_log_level,
        commands::open_recordings_folder,
        commands::open_log_dir,
        commands::open_app_data_dir,
        commands::check_apple_intelligence_available,
        commands::log_from_frontend,
        commands::set_onboarding_paste_override,
        commands::initialize_enigo,
        commands::models::get_available_models,
        commands::models::get_model_info,
        commands::models::download_model,
        commands::models::delete_model,
        commands::models::cancel_download,
        commands::models::set_active_model,
        commands::models::get_current_model,
        commands::models::get_transcription_model_status,
        commands::models::is_model_loading,
        commands::models::has_any_models_available,
        commands::models::has_any_models_or_downloads,
        commands::models::get_recommended_first_model,
        commands::audio::update_microphone_mode,
        commands::audio::get_microphone_mode,
        commands::audio::get_available_microphones,
        commands::audio::set_selected_microphone,
        commands::audio::get_selected_microphone,
        commands::audio::get_available_output_devices,
        commands::audio::set_selected_output_device,
        commands::audio::get_selected_output_device,
        commands::audio::play_test_sound,
        commands::audio::check_custom_sounds,
        commands::audio::set_clamshell_microphone,
        commands::audio::get_clamshell_microphone,
        commands::audio::is_recording,
        commands::audio::start_mic_preview,
        commands::audio::stop_mic_preview,
        commands::transcription::set_model_unload_timeout,
        commands::transcription::get_model_load_status,
        commands::transcription::unload_model_manually,
        commands::history::get_history_entries,
        commands::history::toggle_history_entry_saved,
        commands::history::get_audio_file_path,
        commands::history::delete_history_entry,
        commands::history::update_history_limit,
        commands::history::update_recording_retention_period,
        commands::history::get_home_stats,
        commands::history::clear_all_history,
        commands::history::get_history_storage_usage,
        commands::history::prune_history,
        helpers::clamshell::is_laptop,
        // Permission commands
        permissions::open_accessibility_settings,
        permissions::open_microphone_settings,
        commands::window::show_main_window,
        commands::menu::set_update_menu_text,
        commands::correction::accept_correction,
        commands::correction::dismiss_correction,
        undo::undo_overlay_card_dismissed,
        undo::undo_overlay_card_presented,
        undo::undo_mark_discoverability_hint_seen,
        overlay::overlay_update_interaction_regions,
    ]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    specta_builder
        .export(
            Typescript::default().bigint(BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    // `sentry_guard` lives until `run()` returns (after `builder.run(...)`),
    // keeping the Sentry client active for the full app lifetime.
    let (sentry_guard, sentry_status_message) = initialize_sentry();

    let mut builder = tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always);

    if let Some(client) = sentry_guard.as_ref() {
        builder = builder.plugin(tauri_plugin_sentry::init(client));
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|a| a == "--toggle-transcription") {
                signal_handle::send_transcription_input(app, "transcribe", "CLI");
            } else if args.iter().any(|a| a == "--toggle-post-process") {
                signal_handle::send_transcription_input(app, "transcribe_with_post_process", "CLI");
            } else if args.iter().any(|a| a == "--cancel") {
                crate::utils::cancel_current_operation(app);
            } else {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(ShortcutToggleStates::default()))
        .manage(Mutex::new(false) as OnboardingPasteOverride)
        .manage(undo::UndoManager::default())
        .manage(cli_args.clone())
        .setup(move |app| {
            // Correlation metadata is attached during setup, so any events emitted
            // before this point may not include anon install/run metadata.
            initialize_sentry_identity_scope(app.handle());

            // Initialize tracing with log directory
            let log_dir = app
                .path()
                .app_log_dir()
                .expect("Failed to get app log directory");

            // Create log directory if it doesn't exist
            std::fs::create_dir_all(&log_dir).ok();

            tracing_config::init_tracing(&log_dir).expect("Failed to initialize tracing");
            tracing::info!("{}", sentry_status_message);
            trigger_sentry_privacy_redaction_smoke_if_requested();

            let mut settings = get_settings(app.handle());

            // CLI --debug flag overrides debug_mode and log level (runtime-only, not persisted)
            if cli_args.debug {
                settings.debug_mode = true;
                settings.log_level = settings::LogLevel::Trace;
            }

            // Set file log level from settings
            let file_log_level = match settings.log_level {
                settings::LogLevel::Error => tracing::Level::ERROR,
                settings::LogLevel::Warn => tracing::Level::WARN,
                settings::LogLevel::Info => tracing::Level::INFO,
                settings::LogLevel::Debug => tracing::Level::DEBUG,
                settings::LogLevel::Trace => tracing::Level::TRACE,
            };
            tracing_config::set_file_log_level(file_log_level);

            let file_log_level_log = match settings.log_level {
                settings::LogLevel::Error => log::LevelFilter::Error,
                settings::LogLevel::Warn => log::LevelFilter::Warn,
                settings::LogLevel::Info => log::LevelFilter::Info,
                settings::LogLevel::Debug => log::LevelFilter::Debug,
                settings::LogLevel::Trace => log::LevelFilter::Trace,
            };
            // Store the file log level in the atomic for the filter to use
            FILE_LOG_LEVEL.store(file_log_level_log as u8, Ordering::Relaxed);

            let app_handle = app.handle().clone();
            app.manage(TranscriptionCoordinator::new(app_handle.clone()));

            initialize_core_logic(&app_handle);

            #[cfg(target_os = "macos")]
            if let Err(e) = menu::init(&app_handle) {
                tracing::error!("Failed to initialize app menu: {}", e);
            }

            // Hide tray icon if --no-tray was passed
            if cli_args.no_tray {
                tray::set_tray_visibility(&app_handle, false);
            }

            // Show main window only if not starting hidden
            // CLI --start-hidden flag overrides the setting
            let should_hide = settings.start_hidden || cli_args.start_hidden;
            if !should_hide {
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    main_window.show().unwrap();
                    main_window.set_focus().unwrap();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let settings = get_settings(&window.app_handle());
                let cli = window.app_handle().state::<CliArgs>();
                // If tray icon is hidden (via setting or --no-tray flag), quit the app
                if !settings.show_tray_icon || cli.no_tray {
                    window.app_handle().exit(0);
                    return;
                }
                api.prevent_close();
                let _res = window.hide();
                #[cfg(target_os = "macos")]
                {
                    let res = window
                        .app_handle()
                        .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    if let Err(e) = res {
                        tracing::error!("Failed to set activation policy: {}", e);
                    }
                }
            }
            tauri::WindowEvent::ThemeChanged(theme) => {
                tracing::info!("Theme changed to: {:?}", theme);
                // Update tray icon to match new theme, maintaining idle state
                utils::change_tray_icon(window.app_handle(), utils::TrayIconState::Idle);
            }
            tauri::WindowEvent::Focused(focused) => {
                if *focused && window.label() == "main" {
                    undo::flush_pending_linux_toast(&window.app_handle());
                }
            }
            _ => {}
        })
        .invoke_handler(specta_builder.invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen {
                    has_visible_windows,
                    ..
                } => {
                    if !has_visible_windows {
                        let remaining_ms = utils::cancel_reopen_suppression_remaining_ms();
                        let cancel_suppressed = utils::is_cancel_reopen_suppressed();
                        if overlay::is_overlay_active() {
                            tracing::info!(
                                event_code = "reopen_foreground_suppressed",
                                reason = "overlay_active",
                                remaining_ms,
                                "App reopen requested with no visible windows while overlay is active, skipping main window show"
                            );
                        } else if cancel_suppressed {
                            tracing::info!(
                                event_code = "reopen_foreground_suppressed",
                                reason = "cancel_suppression",
                                remaining_ms,
                                "App reopen requested with no visible windows during cancel suppression window, skipping main window show"
                            );
                        } else {
                            tracing::info!(
                                "App reopen requested with no visible windows, showing main window"
                            );
                            show_main_window(app);
                        }
                    }
                }
                // Handle app exit to properly shutdown sidecar processes
                tauri::RunEvent::Exit => {
                    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                    {
                        if let Some(mlx_manager) = app.try_state::<Arc<MlxModelManager>>() {
                            tracing::info!("App exiting, stopping MLX sidecar server...");
                            if let Err(e) = mlx_manager.stop_server() {
                                tracing::error!("Failed to stop MLX sidecar server: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{is_sensitive_key, scrub_sentry_event};
    use sentry::protocol::{Event as SentryEvent, User};

    #[test]
    fn scrubber_preserves_anon_user_id() {
        let mut event = SentryEvent::default();
        event.user = Some(User {
            id: Some("anon:550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        });

        let scrubbed = scrub_sentry_event(event);
        let user = scrubbed.user.expect("user should exist");
        assert_eq!(
            user.id.as_deref(),
            Some("anon:550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn scrubber_redacts_non_anon_user_id() {
        let mut event = SentryEvent::default();
        event.user = Some(User {
            id: Some("john@example.com".to_string()),
            ..Default::default()
        });

        let scrubbed = scrub_sentry_event(event);
        let user = scrubbed.user.expect("user should exist");
        assert_eq!(user.id.as_deref(), Some("[redacted-user-id]"));
    }

    #[test]
    fn scrubber_redacts_non_anon_user_id_that_matches_no_pattern() {
        let mut event = SentryEvent::default();
        event.user = Some(User {
            id: Some("employee_12345".to_string()),
            ..Default::default()
        });

        let scrubbed = scrub_sentry_event(event);
        let user = scrubbed.user.expect("user should exist");
        assert_eq!(user.id.as_deref(), Some("[redacted-user-id]"));
    }

    #[test]
    fn sensitive_key_token_matching_avoids_common_false_positives() {
        assert!(!is_sensitive_key("tokenizer"));
        assert!(!is_sensitive_key("tokenization_mode"));
    }

    #[test]
    fn sensitive_key_token_and_credential_variants_are_detected() {
        assert!(is_sensitive_key("token"));
        assert!(is_sensitive_key("access_token"));
        assert!(is_sensitive_key("accessToken"));
        assert!(is_sensitive_key("api_credentials"));
    }
}
