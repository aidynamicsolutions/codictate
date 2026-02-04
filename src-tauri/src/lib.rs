mod actions;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple_intelligence;
mod audio_device_info;
mod audio_feedback;
pub mod audio_toolkit;
mod clipboard;
mod commands;
#[cfg(target_os = "macos")]
mod fn_key_monitor;
mod helpers;
mod i18n;
mod input;
mod llm_client;
mod managers;
mod notification;
mod overlay;
mod permissions;
mod settings;
mod shortcut;
mod signal_handle;
mod tray;
mod tray_i18n;
mod tracing_config;
mod user_profile;
mod utils;
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{collect_commands, Builder};
use managers::audio::AudioRecordingManager;
use managers::history::HistoryManager;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use managers::mlx::MlxModelManager;
use managers::model::ModelManager;
use managers::transcription::TranscriptionManager;
#[cfg(unix)]
use signal_hook::consts::SIGUSR2;
#[cfg(unix)]
use signal_hook::iterator::Signals;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::image::Image;

use tauri::tray::TrayIconBuilder;
use tauri::Emitter;
use tauri::{AppHandle, Listener, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::settings::get_settings;


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
    
    // Pre-warm Bluetooth microphone if selected
    // (triggers A2DP→HFP switch if needed)
    recording_manager.prewarm_bluetooth_mic();
    
    // Warm up the recorder (loads VAD model) without opening the mic
    // This removes the ~700ms delay on first record
    recording_manager.warmup_recorder();
    
    // Start background loading of the transcription model
    // This removes the ~1.5s delay on first transcription
    transcription_manager.initiate_model_load();

    // Note: Shortcuts are NOT initialized here.
    // The frontend is responsible for calling the `initialize_shortcuts` command
    // after permissions are confirmed (on macOS) or after onboarding completes.
    // This matches the pattern used for Enigo initialization.

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
    let signals = Signals::new(&[SIGUSR2]).unwrap();
    // Set up SIGUSR2 signal handler for toggling transcription
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

    // Get the autostart manager and configure based on user setting
    let autostart_manager = app_handle.autolaunch();
    let settings = settings::get_settings(&app_handle);

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
pub fn run() {

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
        shortcut::change_app_language_setting,
        shortcut::change_update_checks_setting,
        user_profile::get_user_profile_command,
        user_profile::update_user_profile_setting,
        trigger_update_check,
        commands::cancel_operation,
        commands::get_app_dir_path,
        commands::get_app_settings,
        commands::get_default_settings,
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
        shortcut::change_app_language_setting,
        shortcut::change_update_checks_setting,
        user_profile::get_user_profile_command,
        user_profile::update_user_profile_setting,
        trigger_update_check,
        commands::cancel_operation,
        commands::get_app_dir_path,
        commands::get_app_settings,
        commands::get_default_settings,
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
    ]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    specta_builder
        .export(
            Typescript::default().bigint(BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    let mut builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
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
        .manage(Mutex::new(ShortcutToggleStates::default()))
        .manage(Mutex::new(false) as OnboardingPasteOverride)
        .setup(move |app| {
            // Initialize tracing with log directory
            let log_dir = app.path().app_log_dir()
                .expect("Failed to get app log directory");
            
            // Create log directory if it doesn't exist
            std::fs::create_dir_all(&log_dir).ok();
            
            tracing_config::init_tracing(&log_dir)
                .expect("Failed to initialize tracing");
            
            let settings = get_settings(&app.handle());
            // Set file log level from settings
            let file_log_level = match settings.log_level {
                settings::LogLevel::Error => tracing::Level::ERROR,
                settings::LogLevel::Warn => tracing::Level::WARN,
                settings::LogLevel::Info => tracing::Level::INFO,
                settings::LogLevel::Debug => tracing::Level::DEBUG,
                settings::LogLevel::Trace => tracing::Level::TRACE,
            };
            tracing_config::set_file_log_level(file_log_level);
            
            let app_handle = app.handle().clone();

            initialize_core_logic(&app_handle);



            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
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
                utils::change_tray_icon(&window.app_handle(), utils::TrayIconState::Idle);
            }
            _ => {}
        })
        .invoke_handler(specta_builder.invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Handle app exit to properly shutdown sidecar processes
            if let tauri::RunEvent::Exit = event {
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
        });
}
