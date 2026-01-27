#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::apple_intelligence;
use crate::audio_feedback::{play_feedback_sound, play_feedback_sound_blocking, SoundType};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::history::HistoryManager;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::managers::mlx::MlxModelManager;
use crate::managers::transcription::TranscriptionManager;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::settings::LOCAL_MLX_PROVIDER_ID;
use crate::settings::{get_settings, AppSettings, APPLE_INTELLIGENCE_PROVIDER_ID};
use crate::shortcut;
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils::{self, show_recording_overlay, show_transcribing_overlay};
use crate::ManagedToggleState;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tauri::AppHandle;
use tauri::Manager;
use tracing::{debug, error, info, info_span};

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
struct TranscribeAction;

async fn maybe_post_process_transcription(
    app: &AppHandle,
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    if !settings.post_process_enabled {
        return None;
    }

    let provider = match settings.active_post_process_provider().cloned() {
        Some(provider) => provider,
        None => {
            debug!("Post-processing enabled but no provider is selected");
            return None;
        }
    };

    let model = settings
        .post_process_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    if model.trim().is_empty() {
        debug!(
            "Post-processing skipped because provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    let selected_prompt_id = match &settings.post_process_selected_prompt_id {
        Some(id) => id.clone(),
        None => {
            debug!("Post-processing skipped because no prompt is selected");
            return None;
        }
    };

    let prompt = match settings
        .post_process_prompts
        .iter()
        .find(|prompt| prompt.id == selected_prompt_id)
    {
        Some(prompt) => prompt.prompt.clone(),
        None => {
            debug!(
                "Post-processing skipped because prompt '{}' was not found",
                selected_prompt_id
            );
            return None;
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return None;
    }

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    // Replace ${output} variable in the prompt with the actual text
    let processed_prompt = prompt.replace("${output}", transcription);
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            if !apple_intelligence::check_apple_intelligence_availability() {
                debug!("Apple Intelligence selected but not currently available on this device");
                return None;
            }

            let token_limit = model.trim().parse::<i32>().unwrap_or(0);
            return match apple_intelligence::process_text(&processed_prompt, token_limit) {
                Ok(result) => {
                    if result.trim().is_empty() {
                        debug!("Apple Intelligence returned an empty response");
                        None
                    } else {
                        debug!(
                            "Apple Intelligence post-processing succeeded. Output length: {} chars",
                            result.len()
                        );
                        Some(result)
                    }
                }
                Err(err) => {
                    error!("Apple Intelligence post-processing failed: {}", err);
                    None
                }
            };
        }

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            debug!("Apple Intelligence provider selected on unsupported platform");
            return None;
        }
    }

    // Handle MLX Local AI provider
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if provider.id == LOCAL_MLX_PROVIDER_ID {
        debug!("Using MLX Local AI for post-processing");
        debug!(
            "MLX Local AI post-processing requested with prompt length: {} chars",
            processed_prompt.len()
        );

        // Get the MLX manager from app state
        let mlx_manager = app.state::<Arc<MlxModelManager>>();
        return match mlx_manager.process_text(&processed_prompt).await {
            Ok(result) => {
                if result.trim().is_empty() {
                    debug!("MLX Local AI returned an empty response");
                    None
                } else {
                    debug!(
                        "MLX Local AI post-processing succeeded. Output length: {} chars",
                        result.len()
                    );
                    Some(result)
                }
            }
            Err(e) => {
                error!("MLX Local AI post-processing failed: {}", e);
                // Show native notification so user sees error even if UI is not active
                crate::notification::show_error(app, "errors.postProcessFailed");
                None
            }
        };
    }

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    // Send the chat completion request
    match crate::llm_client::send_chat_completion(&provider, api_key, &model, processed_prompt)
        .await
    {
        Ok(Some(content)) => {
            // Strip invisible Unicode characters that some LLMs (e.g., Qwen) may insert
            let content = content
                .replace('\u{200B}', "") // Zero-Width Space
                .replace('\u{200C}', "") // Zero-Width Non-Joiner
                .replace('\u{200D}', "") // Zero-Width Joiner
                .replace('\u{FEFF}', ""); // Byte Order Mark / Zero-Width No-Break Space
            debug!(
                "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                provider.id,
                content.len()
            );
            Some(content)
        }
        Ok(None) => {
            error!("LLM API response has no content");
            None
        }
        Err(e) => {
            error!(
                "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                provider.id,
                e
            );
            None
        }
    }
}

async fn maybe_convert_chinese_variant(
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    // Check if language is set to Simplified or Traditional Chinese
    let is_simplified = settings.selected_language == "zh-Hans";
    let is_traditional = settings.selected_language == "zh-Hant";

    if !is_simplified && !is_traditional {
        debug!("selected_language is not Simplified or Traditional Chinese; skipping translation");
        return None;
    }

    debug!(
        "Starting Chinese translation using OpenCC for language: {}",
        settings.selected_language
    );

    // Use OpenCC to convert based on selected language
    let config = if is_simplified {
        // Convert Traditional Chinese to Simplified Chinese
        BuiltinConfig::Tw2sp
    } else {
        // Convert Simplified Chinese to Traditional Chinese
        BuiltinConfig::S2twp
    };

    match OpenCC::from_config(config) {
        Ok(converter) => {
            let converted = converter.convert(transcription);
            debug!(
                "OpenCC translation completed. Input length: {}, Output length: {}",
                transcription.len(),
                converted.len()
            );
            Some(converted)
        }
        Err(e) => {
            error!("Failed to initialize OpenCC converter: {}. Falling back to original transcription.", e);
            None
        }
    }
}

impl ShortcutAction for TranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // Generate session ID for log correlation
        let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let _session_span = info_span!("session", session = %session_id).entered();
        
        // Emit session ID to frontend for correlated logging
        use tauri::Emitter;
        let _ = app.emit("session-started", &session_id);
        
        // Check microphone permission BEFORE showing any UI
        // This prevents the overlay from appearing when permission is denied
        #[cfg(target_os = "macos")]
        {
            use crate::permissions::{check_microphone_permission, MicrophonePermission};
            
            if check_microphone_permission() == MicrophonePermission::Denied {
                error!("Microphone permission denied, cannot start recording");
                
                // Show the main window so the permission dialog modal can be seen
                // (the app usually runs in the background)
                crate::show_main_window(app);
                
                // Emit event to frontend to show permission dialog
                let _ = app.emit("microphone-permission-denied", ());
                return; // Don't show overlay or start recording
            }
        }
        
        let start_time = Instant::now();
        info!("Recording started for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        tm.initiate_model_load();

        let binding_id = binding_id.to_string();
        change_tray_icon(app, TrayIconState::Recording);
        
        show_recording_overlay(app);

        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Get the microphone mode to determine audio feedback timing
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;
        debug!("Microphone mode - always_on: {}", is_always_on);

        let mut recording_started = false;
        if is_always_on {
            // Always-on mode: Play audio feedback immediately, then apply mute after sound finishes
            debug!("Always-on mode: Playing audio feedback immediately");
            let rm_clone = Arc::clone(&rm);
            let app_clone = app.clone();
            // The blocking helper exits immediately if audio feedback is disabled,
            // so we can always reuse this thread to ensure mute happens right after playback.
            std::thread::spawn(move || {
                play_feedback_sound_blocking(&app_clone, SoundType::Start);
                rm_clone.apply_mute();
            });

            recording_started = rm.try_start_recording(&binding_id, &session_id);
            debug!("Recording started: {}", recording_started);
        } else {
            // On-demand mode: Start recording first, then play audio feedback, then apply mute
            // This allows the microphone to be activated before playing the sound
            debug!("On-demand mode: Starting recording first, then audio feedback");
            let recording_start_time = Instant::now();
            if rm.try_start_recording(&binding_id, &session_id) {
                recording_started = true;
                debug!("Recording started in {:?}", recording_start_time.elapsed());
                // Small delay to ensure microphone stream is active
                let app_clone = app.clone();
                let rm_clone = Arc::clone(&rm);
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    debug!("Handling delayed audio feedback/mute sequence");
                    // Helper handles disabled audio feedback by returning early, so we reuse it
                    // to keep mute sequencing consistent in every mode.
                    play_feedback_sound_blocking(&app_clone, SoundType::Start);
                    rm_clone.apply_mute();
                });
            } else {
                debug!("Failed to start recording");
            }
        }

        if recording_started {
            // Dynamically register the cancel shortcut in a separate task to avoid deadlock
            shortcut::register_cancel_shortcut(app);
        }

        debug!(
            "TranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // Unregister the cancel shortcut when transcription stops
        shortcut::unregister_cancel_shortcut(app);

        let stop_time = Instant::now();
        debug!("TranscribeAction::stop called for binding: {}", binding_id);

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        change_tray_icon(app, TrayIconState::Transcribing);
        
        show_transcribing_overlay(app);

        // Unmute before playing audio feedback so the stop sound is audible
        rm.remove_mute();

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        // Get session_id BEFORE stopping (it will be cleared after stop)
        let session_id = rm.get_current_session_id().unwrap_or_else(|| "unknown".to_string());
        
        let binding_id = binding_id.to_string(); // Clone binding_id for the async task

        tauri::async_runtime::spawn({
            let session_span = info_span!("session", session = %session_id);
            async move {
                let _guard = session_span.enter();
                let binding_id = binding_id.clone();
                debug!(
                    "Starting async transcription task for binding: {}",
                    binding_id
                );

                let stop_recording_time = Instant::now();
                if let Some(samples) = rm.stop_recording(&binding_id) {
                    debug!(
                        "Recording stopped and samples retrieved in {:?}, sample count: {}",
                        stop_recording_time.elapsed(),
                        samples.len()
                    );

                    let transcription_time = Instant::now();
                    let samples_clone = samples.clone();
                    match tm.transcribe(samples) {
                        Ok(transcription) => {
                            debug!(
                                "Transcription completed in {:?}: '{}'",
                                transcription_time.elapsed(),
                                transcription
                            );
                            if !transcription.is_empty() {
                                let settings = get_settings(&ah);
                                let mut final_text = transcription.clone();
                                let mut post_processed_text: Option<String> = None;
                                let mut post_process_prompt: Option<String> = None;

                                // First, check if Chinese variant conversion is needed
                                if let Some(converted_text) =
                                    maybe_convert_chinese_variant(&settings, &transcription).await
                                {
                                    final_text = converted_text;
                                }

                                // Then apply regular post-processing if enabled
                                // Uses final_text which may already have Chinese conversion applied
                                if let Some(processed_text) =
                                    {
                                        utils::show_processing_overlay(&ah);
                                        maybe_post_process_transcription(&ah, &settings, &final_text).await
                                    }
                                {
                                    post_processed_text = Some(processed_text.clone());
                                    final_text = processed_text;

                                    if let Some(prompt_id) = &settings.post_process_selected_prompt_id {
                                        if let Some(prompt) = settings
                                            .post_process_prompts
                                            .iter()
                                            .find(|p| &p.id == prompt_id)
                                        {
                                            post_process_prompt = Some(prompt.prompt.clone());
                                        }
                                    }
                                } else if final_text != transcription {
                                    // Chinese conversion was applied but no LLM post-processing
                                    post_processed_text = Some(final_text.clone());
                                }

                                let hm_clone = Arc::clone(&hm);
                                let transcription_for_history = transcription.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = hm_clone
                                        .save_transcription(
                                            samples_clone,
                                            transcription_for_history,
                                            post_processed_text,
                                            post_process_prompt,
                                        )
                                        .await
                                    {
                                        error!("Failed to save transcription to history: {}", e);
                                    }
                                });

                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                ah.run_on_main_thread(move || {
                                    match utils::paste(final_text, ah_clone.clone()) {
                                        Ok(()) => debug!(
                                            "Text pasted successfully in {:?}",
                                            paste_time.elapsed()
                                        ),
                                        Err(e) => error!("Failed to paste transcription: {}", e),
                                    }
                                    utils::hide_recording_overlay(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
                                })
                                .unwrap_or_else(|e| {
                                    error!("Failed to run paste on main thread: {:?}", e);
                                    utils::hide_recording_overlay(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                });
                            } else {
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                            }
                        }
                        Err(err) => {
                            debug!("Global Shortcut Transcription error: {}", err);
                            utils::hide_recording_overlay(&ah);
                            change_tray_icon(&ah, TrayIconState::Idle);
                        }
                    }
                } else {
                    debug!("No samples retrieved from recording stop");
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                }

                // Clear toggle state now that transcription is complete
                if let Ok(mut states) = ah.state::<ManagedToggleState>().lock() {
                    states.active_toggles.insert(binding_id, false);
                }
            }
        });

        debug!(
            "TranscribeAction::stop completed in {:?}",
            stop_time.elapsed()
        );
    }
}

// Cancel Action
struct CancelAction;

impl ShortcutAction for CancelAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        utils::cancel_current_operation(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on stop for cancel
    }
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        info!(
            "Shortcut ID '{}': Started - {} (App: {})",
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        info!(
            "Shortcut ID '{}': Stopped - {} (App: {})",
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// Paste Last Transcript Action
struct PasteLastTranscriptAction;

impl ShortcutAction for PasteLastTranscriptAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        debug!("Paste last transcript triggered");
        
        // Check if recording is active - don't interfere with recording/transcription flow
        let audio_manager = app.state::<Arc<AudioRecordingManager>>();
        if audio_manager.is_recording() {
            debug!("Paste last transcript skipped: recording is active");
            return;
        }
        
        let app_clone = app.clone();
        
        // Spawn async task to get the latest transcription and paste it
        tauri::async_runtime::spawn(async move {
            let text = if let Some(history_manager) = app_clone.try_state::<Arc<HistoryManager>>() {
                let manager = history_manager.inner().clone();
                match manager.get_history_entries().await {
                    Ok(entries) => {
                        if let Some(latest) = entries.first() {
                            // Prefer post-processed text if available, otherwise use raw transcription
                            let text = latest
                                .post_processed_text
                                .as_ref()
                                .unwrap_or(&latest.transcription_text)
                                .clone();
                            debug!(chars = text.len(), "Retrieved latest transcription for paste");
                            Some(text)
                        } else {
                            info!("No history entries available for paste last transcript");
                            // Show notification to inform user
                            crate::notification::show_info(&app_clone, "pasteLastTranscript.noHistory");
                            None
                        }
                    }
                    Err(e) => {
                        error!("Failed to get history entries: {}", e);
                        None
                    }
                }
            } else {
                error!("HistoryManager not available for paste last transcript");
                None
            };
            
            // If we have text, paste it on the main thread
            if let Some(text_to_paste) = text {
                if !text_to_paste.is_empty() {
                    // Add a delay before pasting to allow the user to release the hotkey modifiers.
                    // When pressing e.g. Ctrl+Cmd+V, those keys are still held when we try to
                    // simulate Cmd+V. Without this delay, the target app would receive Ctrl+Cmd+V
                    // (which does nothing) instead of just Cmd+V.
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    
                    let app_for_paste = app_clone.clone();
                    if let Err(e) = app_clone.run_on_main_thread(move || {
                        match crate::utils::paste(text_to_paste, app_for_paste) {
                            Ok(()) => info!("Pasted last transcript successfully"),
                            Err(e) => error!("Failed to paste last transcript: {}", e),
                        }
                    }) {
                        error!("Failed to run paste on main thread: {:?}", e);
                    }
                } else {
                    debug!("Paste last transcript skipped: transcription text is empty");
                }
            }
        });
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on key release
    }
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_handsfree".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "cancel".to_string(),
        Arc::new(CancelAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "paste_last_transcript".to_string(),
        Arc::new(PasteLastTranscriptAction) as Arc<dyn ShortcutAction>,
    );
    map
});
