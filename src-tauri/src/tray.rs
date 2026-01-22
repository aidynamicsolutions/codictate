use crate::managers::history::HistoryManager;
use crate::settings;
use crate::tray_i18n::get_tray_translations;
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Manager, Theme};


#[derive(Clone, Debug, PartialEq)]
pub enum TrayIconState {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppTheme {
    Dark,
    Light,
    Colored, // Pink/colored theme for Linux
}

/// Gets the current app theme, with Linux defaulting to Colored theme
pub fn get_current_theme(app: &AppHandle) -> AppTheme {
    if cfg!(target_os = "linux") {
        // On Linux, always use the colored theme
        AppTheme::Colored
    } else {
        // On other platforms, map system theme to our app theme
        if let Some(main_window) = app.get_webview_window("main") {
            match main_window.theme().unwrap_or(Theme::Dark) {
                Theme::Light => AppTheme::Light,
                Theme::Dark => AppTheme::Dark,
                _ => AppTheme::Dark, // Default fallback
            }
        } else {
            AppTheme::Dark
        }
    }
}

/// Gets the appropriate icon path for the given theme and state
pub fn get_icon_path(theme: AppTheme, state: TrayIconState) -> &'static str {
    match (theme, state) {
        // Dark theme uses light icons
        (AppTheme::Dark, TrayIconState::Idle) => "resources/tray_idle.png",
        (AppTheme::Dark, TrayIconState::Recording) => "resources/tray_recording.png",
        (AppTheme::Dark, TrayIconState::Transcribing) => "resources/tray_transcribing.png",
        // Light theme uses dark icons
        (AppTheme::Light, TrayIconState::Idle) => "resources/tray_idle_dark.png",
        (AppTheme::Light, TrayIconState::Recording) => "resources/tray_recording_dark.png",
        (AppTheme::Light, TrayIconState::Transcribing) => "resources/tray_transcribing_dark.png",
        // Colored theme uses pink icons (for Linux)
        (AppTheme::Colored, TrayIconState::Idle) => "resources/codictate.png",
        (AppTheme::Colored, TrayIconState::Recording) => "resources/recording.png",
        (AppTheme::Colored, TrayIconState::Transcribing) => "resources/transcribing.png",
    }
}

pub fn change_tray_icon(app: &AppHandle, icon: TrayIconState) {
    let tray = app.state::<TrayIcon>();
    let theme = get_current_theme(app);

    let icon_path = get_icon_path(theme, icon.clone());

    // Use graceful error handling instead of expect() to prevent panics
    // Panics in the TapDisabled callback thread can cause keyboard/mouse lockup
    match app.path().resolve(icon_path, tauri::path::BaseDirectory::Resource) {
        Ok(resolved_path) => {
            match Image::from_path(&resolved_path) {
                Ok(image) => {
                    let _ = tray.set_icon(Some(image));
                }
                Err(e) => {
                    tracing::error!("Failed to load tray icon from {:?}: {}", resolved_path, e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to resolve tray icon path '{}': {}", icon_path, e);
        }
    }

    // Update menu based on state
    // For Idle state, spawn the menu update in a background task to avoid blocking
    // (the Idle menu may query history, which requires async DB access)
    match icon {
        TrayIconState::Idle => {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                update_tray_menu_async(&app_clone, &TrayIconState::Idle, None).await;
            });
        }
        _ => {
            // Recording/Transcribing states don't need history, can use sync update
            update_tray_menu_sync(app, &icon, None);
        }
    }
}

/// Async version of update_tray_menu - use this when calling from async context
pub async fn update_tray_menu_async(app: &AppHandle, state: &TrayIconState, locale: Option<&str>) {
    let has_history = if matches!(state, TrayIconState::Idle) {
        has_history_entries_async(app).await
    } else {
        false
    };
    build_and_set_tray_menu(app, state, locale, has_history);
}

/// Sync version of update_tray_menu - use for Recording/Transcribing states that don't need history
pub fn update_tray_menu_sync(app: &AppHandle, state: &TrayIconState, locale: Option<&str>) {
    // Sync version doesn't check history - only use for non-Idle states
    build_and_set_tray_menu(app, state, locale, false);
}



/// Internal function that builds and sets the tray menu
fn build_and_set_tray_menu(
    app: &AppHandle,
    state: &TrayIconState,
    locale: Option<&str>,
    has_history: bool,
) {
    let settings = settings::get_settings(app);

    let locale = locale.unwrap_or(&settings.app_language);
    let strings = get_tray_translations(Some(locale.to_string()));

    // Platform-specific accelerators
    #[cfg(target_os = "macos")]
    let (settings_accelerator, quit_accelerator) = (Some("Cmd+,"), Some("Cmd+Q"));
    #[cfg(not(target_os = "macos"))]
    let (settings_accelerator, quit_accelerator) = (Some("Ctrl+,"), Some("Ctrl+Q"));

    // Create common menu items
    let version_label = if cfg!(debug_assertions) {
        format!("Codictate v{} (Dev)", env!("CARGO_PKG_VERSION"))
    } else {
        format!("Codictate v{}", env!("CARGO_PKG_VERSION"))
    };
    let version_i = MenuItem::with_id(app, "version", &version_label, false, None::<&str>)
        .expect("failed to create version item");
    let settings_i = MenuItem::with_id(
        app,
        "settings",
        &strings.settings,
        true,
        settings_accelerator,
    )
    .expect("failed to create settings item");
    let check_updates_i = MenuItem::with_id(
        app,
        "check_updates",
        &strings.check_updates,
        settings.update_checks_enabled,
        None::<&str>,
    )
    .expect("failed to create check updates item");
    let quit_i = MenuItem::with_id(app, "quit", &strings.quit, true, quit_accelerator)
        .expect("failed to create quit item");
    let separator = || PredefinedMenuItem::separator(app).expect("failed to create separator");

    let menu = match state {
        TrayIconState::Recording | TrayIconState::Transcribing => {
            let cancel_i = MenuItem::with_id(app, "cancel", &strings.cancel, true, None::<&str>)
                .expect("failed to create cancel item");
            Menu::with_items(
                app,
                &[
                    &version_i,
                    &separator(),
                    &cancel_i,
                    &separator(),
                    &settings_i,
                    &check_updates_i,
                    &separator(),
                    &quit_i,
                ],
            )
            .expect("failed to create menu")
        }
        TrayIconState::Idle => {
            if has_history {
                let copy_last_i = MenuItem::with_id(
                    app,
                    "copy_last_recording",
                    &strings.copy_last_recording,
                    true,
                    None::<&str>,
                )
                .expect("failed to create copy last recording item");
                Menu::with_items(
                    app,
                    &[
                        &version_i,
                        &separator(),
                        &copy_last_i,
                        &separator(),
                        &settings_i,
                        &check_updates_i,
                        &separator(),
                        &quit_i,
                    ],
                )
                .expect("failed to create menu")
            } else {
                Menu::with_items(
                    app,
                    &[
                        &version_i,
                        &separator(),
                        &settings_i,
                        &check_updates_i,
                        &separator(),
                        &quit_i,
                    ],
                )
                .expect("failed to create menu")
            }
        }
    };

    let tray = app.state::<TrayIcon>();
    let _ = tray.set_menu(Some(menu));
    let _ = tray.set_icon_as_template(true);
}

/// Async check if there are any history entries
async fn has_history_entries_async(app: &AppHandle) -> bool {
    if let Some(history_manager) = app.try_state::<Arc<HistoryManager>>() {
        let manager = history_manager.inner().clone();
        match manager.get_history_entries().await {
            Ok(entries) => !entries.is_empty(),
            Err(e) => {
                tracing::warn!("Failed to check history entries: {}", e);
                false
            }
        }
    } else {
        tracing::debug!("HistoryManager not available for history check");
        false
    }
}

/// Copy the latest transcription to clipboard
pub fn copy_last_recording_to_clipboard(app: &AppHandle) {
    if let Some(history_manager) = app.try_state::<Arc<HistoryManager>>() {
        let manager = history_manager.inner().clone();
        let app_clone = app.clone();

        // Spawn async task to get history entries and copy to clipboard
        tauri::async_runtime::spawn(async move {
            match manager.get_history_entries().await {
                Ok(entries) => {
                    if let Some(latest) = entries.first() {
                        // Prefer post-processed text if available, otherwise use raw transcription
                        let text = latest
                            .post_processed_text
                            .as_ref()
                            .unwrap_or(&latest.transcription_text)
                            .clone();

                        // Copy to clipboard on main thread
                        let app_for_clipboard = app_clone.clone();
                        if let Err(e) = app_clone.run_on_main_thread(move || {
                            use tauri_plugin_clipboard_manager::ClipboardExt;
                            let clipboard = app_for_clipboard.clipboard();
                            if let Err(e) = clipboard.write_text(&text) {
                                tracing::error!("Failed to write to clipboard: {}", e);
                            } else {
                                tracing::info!("Copied last recording to clipboard");
                            }
                        }) {
                            tracing::error!("Failed to schedule clipboard copy on main thread: {}", e);
                        }
                    } else {
                        tracing::debug!("No history entries to copy");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get history entries for clipboard copy: {}", e);
                }
            }
        });
    } else {
        tracing::warn!("Cannot copy last recording: HistoryManager not available");
    }
}
