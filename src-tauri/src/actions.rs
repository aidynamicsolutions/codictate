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
use crate::utils::{
    self, show_recording_overlay, show_transcribing_overlay,
};
use crate::ManagedToggleState;
use crate::TranscriptionCoordinator;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tracing::{debug, error, info, info_span, warn};

/// Drop guard that notifies the [`TranscriptionCoordinator`] when the
/// transcription pipeline finishes — whether it completes normally or panics.
struct FinishGuard(AppHandle);
impl Drop for FinishGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.try_state::<TranscriptionCoordinator>() {
            c.notify_processing_finished();
        }
    }
}

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
struct TranscribeAction {
    post_process: bool,
}

/// Field name for structured output JSON schema
const TRANSCRIPTION_FIELD: &str = "transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
fn build_system_prompt(prompt_template: &str) -> String {
    prompt_template.replace("${output}", "").trim().to_string()
}

async fn post_process_transcription(
    app: &AppHandle,
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
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

    utils::show_processing_overlay(app);

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
            return match apple_intelligence::process_text_with_system_prompt("", &processed_prompt, token_limit) {
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
        return match mlx_manager
            .process_text(&processed_prompt, None, None, None)
            .await
        {
            Ok(result) => {
                let result: String = result;
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

    if provider.supports_structured_output {
        debug!("Using structured outputs for provider '{}'", provider.id);

        let system_prompt = build_system_prompt(&prompt);
        let user_content = transcription.to_string();

        // Handle Apple Intelligence separately since it uses native Swift APIs
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if !apple_intelligence::check_apple_intelligence_availability() {
                    debug!(
                        "Apple Intelligence selected but not currently available on this device"
                    );
                    return None;
                }

                let token_limit = model.trim().parse::<i32>().unwrap_or(0);
                return match apple_intelligence::process_text_with_system_prompt(
                    &system_prompt,
                    &user_content,
                    token_limit,
                ) {
                    Ok(result) => {
                        if result.trim().is_empty() {
                            debug!("Apple Intelligence returned an empty response");
                            None
                        } else {
                            let result = strip_invisible_chars(&result);
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

        // Define JSON schema for transcription output
        let json_schema = serde_json::json!({
            "type": "object",
            "properties": {
                (TRANSCRIPTION_FIELD): {
                    "type": "string",
                    "description": "The cleaned and processed transcription text"
                }
            },
            "required": [TRANSCRIPTION_FIELD],
            "additionalProperties": false
        });

        match crate::llm_client::send_chat_completion_with_schema(
            &provider,
            api_key.clone(),
            &model,
            user_content,
            Some(system_prompt),
            Some(json_schema),
        )
        .await
        {
            Ok(Some(content)) => {
                // Parse the JSON response to extract the transcription field
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        if let Some(transcription_value) =
                            json.get(TRANSCRIPTION_FIELD).and_then(|t| t.as_str())
                        {
                            let result = strip_invisible_chars(transcription_value);
                            debug!(
                                "Structured output post-processing succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                result.len()
                            );
                            return Some(result);
                        } else {
                            error!("Structured output response missing 'transcription' field");
                            return Some(strip_invisible_chars(&content));
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse structured output JSON: {}. Returning raw content.",
                            e
                        );
                        return Some(strip_invisible_chars(&content));
                    }
                }
            }
            Ok(None) => {
                error!("LLM API response has no content");
                return None;
            }
            Err(e) => {
                warn!(
                    "Structured output failed for provider '{}': {}. Falling back to legacy mode.",
                    provider.id, e
                );
                // Fall through to legacy mode below
            }
        }
    }

    // Legacy mode: Replace ${output} variable in the prompt with the actual text
    let processed_prompt = prompt.replace("${output}", transcription);
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    match crate::llm_client::send_chat_completion(&provider, api_key, &model, processed_prompt)
        .await
    {
        Ok(Some(content)) => {
            let content = strip_invisible_chars(&content);
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

fn auto_refined_from_post_processed_text(post_processed_text: &Option<String>) -> bool {
    post_processed_text.is_some()
}

fn build_undo_paste_capture(
    source_action: &'static str,
    stats_token: Option<u64>,
    auto_refined: bool,
    paste_result: crate::clipboard::PasteResult,
    suggestion_text: String,
) -> crate::undo::PasteCapture {
    crate::undo::PasteCapture {
        source_action,
        stats_token,
        auto_refined,
        pasted_text: paste_result.pasted_text,
        suggestion_text,
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
        crate::undo::clear_stop_transition_marker(app);

        let app_clone = app.clone();
        let binding_id_clone = binding_id.to_string();

        // Synchronously prepare recording state to handle race conditions
        // This sets the state to "Preparing" so if stop() is called immediately,
        // it can cancel the start operation.
        let rm = app.state::<Arc<AudioRecordingManager>>();
        if !rm.prepare_recording(binding_id) {
            debug!(
                "Failed to prepare recording for binding {} (state not Idle)",
                binding_id
            );
            return;
        }

        // Spawn a thread to avoid blocking the event tap/caller
        std::thread::spawn(move || {
            let app = &app_clone;
            let binding_id = &binding_id_clone;

            // Check microphone permission BEFORE showing any UI
            // This prevents the overlay from appearing when permission is denied
            #[cfg(target_os = "macos")]
            {
                use crate::permissions::{check_microphone_permission, MicrophonePermission};

                if check_microphone_permission() == MicrophonePermission::Denied {
                    error!("Microphone permission denied, cannot start recording");

                    // Emit event so frontend can show permission UI if it's already visible
                    let _ = app.emit("microphone-permission-denied", ());
                    // Show native notification without stealing focus from the active app
                    crate::notification::show_microphone_permission_denied(app);
                    return; // Don't show overlay or start recording
                }
            }

            let start_time = Instant::now();
            info!("Recording started for binding: {}", binding_id);

            // Load model in the background
            let tm = app.state::<Arc<TranscriptionManager>>();
            tm.initiate_model_load();

            change_tray_icon(app, TrayIconState::Recording);

            let rm = app.state::<Arc<AudioRecordingManager>>();

            // Get the microphone mode to determine audio feedback timing
            let settings = get_settings(app);
            let is_always_on = settings.always_on_microphone;
            debug!("Microphone mode - always_on: {}", is_always_on);

            let mut recording_started = false;

            // NOTE: try_start_recording is now BLOCKING until the audio stream is ready (in OnDemand mode).
            // This ensures we don't show the overlay until we are actually recording.

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

                if rm.try_start_recording(binding_id, &session_id) {
                    recording_started = true;
                    // Show overlay only after recording started success
                    info!("TranscribeAction::start: showing recording overlay (always-on)");
                    show_recording_overlay(app);
                } else {
                    debug!("Failed to start recording (always-on)");
                }
            } else {
                // On-demand mode: Start recording first, then play audio feedback, then apply mute
                // This allows the microphone to be activated before playing the sound
                debug!("On-demand mode: Starting recording first (blocking), then audio feedback");

                // Check if we're using a Bluetooth device and if this is the first trigger
                // For Bluetooth: show "Starting microphone..." overlay during warmup
                // For first trigger of ANY device: show "Starting microphone..." while initializing
                // Note: Use catch_unwind to protect against any panics in device detection
                let is_bluetooth = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    rm.is_current_device_bluetooth()
                }))
                .unwrap_or_else(|e| {
                    error!("Panic in Bluetooth detection: {:?}", e);
                    false
                });

                let is_first_trigger = rm.is_first_trigger();

                // Only show "Starting microphone..." for Bluetooth devices.
                // Internal mics start fast enough (~100-200ms) that we can just wait
                // and show the recording overlay directly - no confusing state transitions.
                // Bluetooth mics need 1-2s warmup, so the connecting overlay sets expectations.
                if is_bluetooth {
                    info!("Bluetooth mic detected, showing connecting overlay");
                    utils::show_connecting_overlay(app);
                }

                let recording_start_time = Instant::now();

                // Blocks here until Mic is ready (~100-200ms for internal, ~500ms+ for Bluetooth)
                if rm.try_start_recording(binding_id, &session_id) {
                    recording_started = true;
                    debug!("Recording started in {:?}", recording_start_time.elapsed());

                    // Add warmup delay for Bluetooth microphones.
                    // Bluetooth mics often send silence while waking up, causing first words to be lost.
                    // Pre-warming triggers the Bluetooth profile switch at app startup, so we only need
                    // a short delay here for audio buffer stabilization:
                    // - First trigger (no pre-warm): 1000ms (longer, in case pre-warm didn't happen)
                    // - Subsequent triggers: 750ms (buffer stabilization)
                    if is_bluetooth {
                        let warmup_delay_ms: u64 = if is_first_trigger { 1000 } else { 750 };
                        info!(
                            delay_ms = warmup_delay_ms,
                            is_first_trigger = is_first_trigger,
                            "Bluetooth microphone: adding warmup delay (pre-warmed)"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(warmup_delay_ms));
                        debug!("Bluetooth warmup delay completed");
                    }

                    // Show overlay recording state NOW (with smooth fade-in animation)
                    info!("TranscribeAction::start: showing recording overlay (on-demand)");
                    show_recording_overlay(app);

                    // Small delay to ensure microphone stream is active (extra safety)
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
                    debug!("Failed to start recording (on-demand)");
                    // Ensure icon is reset if we failed
                    change_tray_icon(app, TrayIconState::Idle);
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
        });
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // We delay unregistering cancel shortcut until after transcription or cancellation
        // shortcut::unregister_cancel_shortcut(app);

        let stop_time = Instant::now();
        debug!("TranscribeAction::stop called for binding: {}", binding_id);
        crate::undo::mark_stop_transition_marker(app);

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        change_tray_icon(app, TrayIconState::Transcribing);

        // Unmute before playing audio feedback so the stop sound is audible
        rm.remove_mute();

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        // Get session_id BEFORE stopping (it will be cleared after stop)
        let session_id = rm
            .get_current_session_id()
            .unwrap_or_else(|| "unknown".to_string());

        let binding_id = binding_id.to_string(); // Clone binding_id for the async task
        let session_id_for_task = session_id.clone();
        let post_process = self.post_process;

        tauri::async_runtime::spawn({
            let session_span = info_span!("session", session = %session_id);
            async move {
                let _guard = session_span.enter();
                let _finish_guard = FinishGuard(ah.clone());
                let _stop_transition_guard = crate::undo::StopTransitionGuard::new(&ah);
                let binding_id = binding_id.clone();
                debug!(
                    "Starting async transcription task for binding: {}",
                    binding_id
                );

                let stop_recording_time = Instant::now();
                if let Some(samples) = rm.stop_recording(&binding_id) {
                    // Register this session as active for transcription
                    tm.set_active_session(session_id_for_task.clone());

                    // Start showing transcribing overlay NOW, after we have confirmed valid samples.
                    // This protects against "phantom stops" from double-triggers showing the overlay incorrectly.
                    show_transcribing_overlay(&ah);

                    debug!(
                        "Recording stopped and samples retrieved in {:?}, sample count: {}",
                        stop_recording_time.elapsed(),
                        samples.len()
                    );

                    let transcription_time = Instant::now();
                    let samples_clone = samples.clone(); // Clone for history saving
                    match tm.transcribe(samples) {
                        Ok((transcription, filler_words_removed)) => {
                            // Check if the session was cancelled during transcription (from llm)
                            if !tm.is_session_active(&session_id_for_task) {
                                debug!(
                                    "Transcription for session {} was cancelled, discarding result",
                                    session_id_for_task
                                );
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                                shortcut::unregister_cancel_shortcut(&ah);
                                return;
                            }

                            debug!(
                                "Transcription completed in {:?}: '{}' (filler_words_removed: {})",
                                transcription_time.elapsed(),
                                transcription,
                                filler_words_removed
                            );
                            if !transcription.trim().is_empty() {
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

                                // Check cancellation again before expensive LLM post-processing
                                if !tm.is_session_active(&session_id_for_task) {
                                    debug!(
                                        "Session {} cancelled before post-processing, aborting",
                                        session_id_for_task
                                    );
                                    utils::hide_overlay_after_transcription(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                    shortcut::unregister_cancel_shortcut(&ah);
                                    return;
                                }

                                // Then apply regular post-processing if:
                                // 1. Explicitly requested via transcribe_with_post_process (post_process = true), OR
                                // 2. Auto-refine is enabled (user opted into always refining)
                                let processed = if post_process || settings.auto_refine_enabled {
                                    post_process_transcription(&ah, &settings, &final_text).await
                                } else {
                                    None
                                };
                                if let Some(processed_text) = processed {
                                    post_processed_text = Some(processed_text.clone());
                                    final_text = processed_text;

                                    if let Some(prompt_id) =
                                        &settings.post_process_selected_prompt_id
                                    {
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

                                let source_action = if post_process {
                                    "transcribe_with_post_process"
                                } else {
                                    "transcribe"
                                };
                                let stats_token = crate::undo::reserve_stats_token(&ah);
                                let auto_refined =
                                    auto_refined_from_post_processed_text(&post_processed_text);
                                let suggestion_text = transcription.clone();

                                let hm_clone = Arc::clone(&hm);
                                let transcription_for_history = transcription.clone();
                                let filler_count = filler_words_removed as i64;
                                let app_for_stats = ah.clone();
                                tauri::async_runtime::spawn(async move {
                                    // Calculate duration in milliseconds (sample rate is 16kHz)
                                    let duration_ms =
                                        (samples_clone.len() as f64 / 16000.0 * 1000.0) as i64;
                                    match hm_clone
                                        .save_transcription(
                                            samples_clone,
                                            transcription_for_history,
                                            post_processed_text,
                                            post_process_prompt,
                                            duration_ms,
                                            filler_count,
                                        )
                                        .await
                                    {
                                        Ok(contribution) => {
                                            crate::undo::register_stats_contribution(
                                                &app_for_stats,
                                                stats_token,
                                                source_action,
                                                contribution,
                                            );
                                        }
                                        Err(e) => {
                                            error!("Failed to save transcription to history: {}", e);
                                        }
                                    }
                                });

                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                ah.run_on_main_thread(move || {
                                    match utils::paste(final_text, ah_clone.clone()) {
                                        Ok(result) => {
                                            debug!(
                                                "Text pasted successfully in {:?}",
                                                paste_time.elapsed()
                                            );
                                            if result.did_paste {
                                                let capture = build_undo_paste_capture(
                                                    source_action,
                                                    Some(stats_token),
                                                    auto_refined,
                                                    result,
                                                    suggestion_text.clone(),
                                                );
                                                crate::undo::register_successful_paste(
                                                    &ah_clone,
                                                    capture,
                                                );
                                            }
                                        }
                                        Err(e) => error!("Failed to paste transcription: {}", e),
                                    }
                                    utils::hide_overlay_after_transcription(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
                                })
                                .unwrap_or_else(|e| {
                                    error!("Failed to run paste on main thread: {:?}", e);
                                    utils::hide_overlay_after_transcription(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                });
                            } else {
                                utils::hide_overlay_after_transcription(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                            }
                        }
                        Err(err) => {
                            debug!("Global Shortcut Transcription error: {}", err);
                            utils::hide_overlay_after_transcription(&ah);
                            change_tray_icon(&ah, TrayIconState::Idle);
                        }
                    }
                } else {
                    debug!("No samples retrieved from recording stop");
                    // Use SAFE hide. If another thread is actively transcribing, this "failed stop"
                    // should not interrupt it.
                    utils::hide_overlay_if_recording(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                }

                // Clear active session to ensure unregistration works
                tm.clear_active_session();

                // Clear toggle state now that transcription is complete
                if let Ok(mut states) = ah.state::<ManagedToggleState>().lock() {
                    states.active_toggles.insert(binding_id, false);
                }

                // Always ensure cancel shortcut is unregistered at the end
                shortcut::unregister_cancel_shortcut(&ah);
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

struct UndoLastTranscriptAction;

impl ShortcutAction for UndoLastTranscriptAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        crate::undo::trigger_undo_last_transcript(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // One-shot action; no key-release behavior.
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
                match manager.get_history_entries(1, 0, None, false, None).await {
                    Ok(entries) => {
                        if let Some(latest) = entries.first() {
                            // Prefer post-processed text if available, otherwise use raw transcription
                            let text = latest
                                .post_processed_text
                                .as_ref()
                                .unwrap_or(&latest.transcription_text)
                                .clone();
                            debug!(
                                chars = text.len(),
                                "Retrieved latest transcription for paste"
                            );
                            Some((text, latest.transcription_text.clone()))
                        } else {
                            info!("No history entries available for paste last transcript");
                            // Show notification to inform user
                            crate::notification::show_info(
                                &app_clone,
                                "pasteLastTranscript.noHistory",
                            );
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
            if let Some((text_to_paste, suggestion_text)) = text {
                if !text_to_paste.is_empty() {
                    // Add a delay before pasting to allow the user to release the hotkey modifiers.
                    // When pressing e.g. Ctrl+Cmd+V, those keys are still held when we try to
                    // simulate Cmd+V. Without this delay, the target app would receive Ctrl+Cmd+V
                    // (which does nothing) instead of just Cmd+V.
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                    let app_for_paste = app_clone.clone();
                    let app_for_undo_slot = app_clone.clone();
                    if let Err(e) =
                        app_clone.run_on_main_thread(move || {
                            match crate::utils::paste(text_to_paste, app_for_paste) {
                                Ok(result) => {
                                    info!("Pasted last transcript successfully");
                                    if result.did_paste {
                                        let capture = build_undo_paste_capture(
                                            "paste_last_transcript",
                                            None,
                                            false,
                                            result,
                                            suggestion_text,
                                        );
                                        crate::undo::register_successful_paste(
                                            &app_for_undo_slot,
                                            capture,
                                        );
                                    }
                                }
                                Err(e) => error!("Failed to paste last transcript: {}", e),
                            }
                        })
                    {
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

// Refine Last Transcript Action
// Applies AI post-processing to the last transcription and pastes the refined text
struct RefineLastTranscriptAction;

impl ShortcutAction for RefineLastTranscriptAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        debug!("Refine last transcript triggered");

        // Check if recording is active - don't interfere with recording/transcription flow
        let audio_manager = app.state::<Arc<AudioRecordingManager>>();
        if audio_manager.is_recording() {
            debug!("Refine last transcript skipped: recording is active");
            return;
        }

        let app_clone = app.clone();

        // Spawn async task to get the latest transcription, refine it, and paste
        tauri::async_runtime::spawn(async move {
            // Get the last raw transcription from history
            let raw_text =
                if let Some(history_manager) = app_clone.try_state::<Arc<HistoryManager>>() {
                    let manager = history_manager.inner().clone();
                    match manager.get_history_entries(1, 0, None, false, None).await {
                        Ok(entries) => {
                            if let Some(latest) = entries.first() {
                                // Always use the RAW transcription, not post-processed
                                Some(latest.transcription_text.clone())
                            } else {
                                info!("No history entries available for refine last transcript");
                                crate::notification::show_info(
                                    &app_clone,
                                    "refineLastTranscript.noHistory",
                                );
                                None
                            }
                        }
                        Err(e) => {
                            error!("Failed to get history entries: {}", e);
                            None
                        }
                    }
                } else {
                    error!("HistoryManager not available for refine last transcript");
                    None
                };

            let Some(raw_text) = raw_text else {
                return;
            };

            if raw_text.is_empty() {
                debug!("Refine last transcript skipped: transcription text is empty");
                return;
            }

            // Get settings and run post-processing
            let settings = get_settings(&app_clone);

            // Check if post-processing is enabled and configured
            if !settings.post_process_enabled {
                info!("Refine is disabled. Enable it in settings first.");
                crate::notification::show_info(&app_clone, "refineLastTranscript.disabled");
                return;
            }

            debug!(
                chars = raw_text.len(),
                "Starting refinement of last transcript"
            );

            // Run post-processing
            let refined_text = post_process_transcription(&app_clone, &settings, &raw_text).await;

            // Hide processing overlay
            crate::utils::hide_overlay_after_transcription(&app_clone);

            let final_text = if let Some(refined) = refined_text {
                debug!(chars = refined.len(), "Refinement completed");
                refined
            } else {
                // Post-processing failed or returned nothing - show notification
                info!("Refinement returned no result, using original text");
                crate::notification::show_info(&app_clone, "refineLastTranscript.failed");
                return;
            };

            // Add a delay before pasting to allow the user to release the hotkey modifiers
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Paste the refined text
            let app_for_paste = app_clone.clone();
            let app_for_undo_slot = app_clone.clone();
            if let Err(e) = app_clone.run_on_main_thread(move || {
                match crate::utils::paste(final_text, app_for_paste) {
                    Ok(result) => {
                        info!("Pasted refined transcript successfully");
                        if result.did_paste {
                            let capture = build_undo_paste_capture(
                                "refine_last_transcript",
                                None,
                                true,
                                result,
                                raw_text,
                            );
                            crate::undo::register_successful_paste(
                                &app_for_undo_slot,
                                capture,
                            );
                        }
                    }
                    Err(e) => error!("Failed to paste refined transcript: {}", e),
                }
            }) {
                error!("Failed to run paste on main thread: {:?}", e);
            }
        });
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on key release
    }
}

// Correct Text Action
// Single-press action that captures context from the focused app and sends to AI for correction.
struct CorrectAction;

impl ShortcutAction for CorrectAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        debug!("Correct text action triggered");

        // Check if recording is active - don't interfere
        let audio_manager = app.state::<Arc<AudioRecordingManager>>();
        if audio_manager.is_recording() {
            debug!("Correct text skipped: recording is active");
            return;
        }

        let app_clone = app.clone();

        tauri::async_runtime::spawn(async move {
            use crate::managers::correction::CorrectionManager;

            let correction_manager = app_clone.state::<Arc<CorrectionManager>>();

            // Show processing overlay while correction is running
            utils::show_processing_overlay(&app_clone);

            match correction_manager.run_correction().await {
                Ok(result) => {
                    if result.has_changes {
                        info!(
                            original = result.original,
                            corrected = result.corrected,
                            "Correction has changes, showing overlay"
                        );
                        // Show correction overlay (emits correction-result + state change internally)
                        crate::overlay::show_correction_overlay(&app_clone, &result);
                    } else {
                        info!("No changes detected by AI correction");
                        // Brief notification, then hide
                        let _ = app_clone.emit("correction-no-changes", ());
                        // Auto-dismiss after 1.5 seconds
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        utils::hide_recording_overlay(&app_clone);
                    }
                }
                Err(e) if e == "correction_in_progress" => {
                    // Another correction is already running — hide the processing overlay
                    // we showed at the start, since the first correction will manage its own.
                    debug!("Correction already in progress, ignoring duplicate trigger");
                    utils::hide_recording_overlay(&app_clone);
                }
                Err(e) if e == "no_text" => {
                    info!("No text to correct");
                    let _ = app_clone.emit("correction-no-text", ());
                    // Auto-dismiss after 2 seconds
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    utils::hide_recording_overlay(&app_clone);
                }
                Err(e) if e == "Accessibility permission required" => {
                    warn!("Accessibility permission needed for correction");
                    // Show the main window so the permission dialog can be seen
                    crate::show_main_window(&app_clone);
                    let _ = app_clone.emit("accessibility-permission-needed", ());
                }
                Err(e) => {
                    error!("Correction failed: {}", e);
                    crate::notification::show_error(&app_clone, "errors.correctionFailed");
                    utils::hide_recording_overlay(&app_clone);
                }
            }
        });
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on key release for single-press correction
    }
}

// Static Action Map
pub static ACTION_MAP: LazyLock<HashMap<String, Arc<dyn ShortcutAction>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction {
            post_process: false,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_with_post_process".to_string(),
        Arc::new(TranscribeAction { post_process: true }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_handsfree".to_string(),
        Arc::new(TranscribeAction {
            post_process: false,
        }) as Arc<dyn ShortcutAction>,
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
    map.insert(
        "undo_last_transcript".to_string(),
        Arc::new(UndoLastTranscriptAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "refine_last_transcript".to_string(),
        Arc::new(RefineLastTranscriptAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "correct_text".to_string(),
        Arc::new(CorrectAction) as Arc<dyn ShortcutAction>,
    );
    map
});

#[cfg(test)]
mod tests {
    use super::{auto_refined_from_post_processed_text, build_undo_paste_capture};

    #[test]
    fn auto_refined_is_false_without_post_processed_output() {
        assert!(!auto_refined_from_post_processed_text(&None));
    }

    #[test]
    fn auto_refined_is_true_when_output_exists() {
        assert!(auto_refined_from_post_processed_text(&Some(
            "refined output".to_string()
        )));
    }

    #[test]
    fn undo_payload_uses_transformed_pasted_text() {
        let paste_result = crate::clipboard::PasteResult {
            pasted_text: "normalized output".to_string(),
            did_paste: true,
        };

        let capture = build_undo_paste_capture(
            "transcribe",
            Some(7),
            true,
            paste_result,
            "raw output".to_string(),
        );

        assert_eq!(capture.pasted_text, "normalized output");
        assert_eq!(capture.source_action, "transcribe");
        assert_eq!(capture.stats_token, Some(7));
        assert!(capture.auto_refined);
        assert_eq!(capture.suggestion_text, "raw output");
    }
}
