use tracing::{debug, error, info, warn};
use serde::Serialize;
use specta::Type;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::actions::ACTION_MAP;

use crate::settings::ShortcutBinding;
use crate::settings::{
    self, get_settings, AutoSubmitKey, ClipboardHandling, CustomWordEntry, LLMPrompt, OverlayPosition, PasteMethod,
    SoundTheme, APPLE_INTELLIGENCE_DEFAULT_MODEL_ID, APPLE_INTELLIGENCE_PROVIDER_ID,
};
use crate::tray;
use crate::ManagedToggleState;
use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use std::sync::Arc;

pub fn init_shortcuts(app: &AppHandle) {
    let default_bindings = settings::get_default_settings().bindings;
    let user_settings = settings::load_or_create_app_settings(app);

    debug!("[init_shortcuts] Starting shortcut initialization");

    // Register all default shortcuts, applying user customizations
    for (id, default_binding) in default_bindings {
        if id == "cancel" {
            debug!("[init_shortcuts] Skipping cancel shortcut (registered dynamically)");
            continue; // Skip cancel shortcut, it will be registered dynamically
        }
        let binding = user_settings
            .bindings
            .get(&id)
            .cloned()
            .unwrap_or(default_binding);

        debug!(
            "[init_shortcuts] Processing binding '{}': current='{}', default='{}'",
            id, binding.current_binding, binding.default_binding
        );

        // Skip registration for fn-based shortcuts (handled by fn_key_monitor.rs)
        let binding_lower = binding.current_binding.to_lowercase();
        if binding_lower == "fn"
            || binding_lower.starts_with("fn+")
            || binding_lower.contains("+fn")
        {
            debug!(
                "[init_shortcuts] Skipping fn-based shortcut '{}' for {} (handled by fn_key_monitor)",
                binding.current_binding, id
            );
            continue;
        }

        if let Err(e) = register_shortcut(app, binding.clone()) {
            error!("[init_shortcuts] Failed to register shortcut '{}' for {}: {}", binding.current_binding, id, e);
        } else {
            debug!("[init_shortcuts] Successfully registered shortcut '{}' for {}", binding.current_binding, id);
        }
    }

    debug!("[init_shortcuts] Shortcut initialization complete");
}

#[derive(Serialize, Type)]
pub struct BindingResponse {
    success: bool,
    binding: Option<ShortcutBinding>,
    error: Option<String>,
}

mod reserved;

// ... existing code ...

#[tauri::command]
#[specta::specta]
pub fn change_binding(
    app: AppHandle,
    id: String,
    binding: String,
) -> Result<BindingResponse, String> {
    // Reject empty bindings — every shortcut should have a value
    if binding.trim().is_empty() {
        return Err("Binding cannot be empty".to_string());
    }

    let mut settings = settings::get_settings(&app);

    // Get the binding to modify, or create it from defaults if it doesn't exist
    let binding_to_modify = match settings.bindings.get(&id) {
        Some(binding) => binding.clone(),
        None => {
            // Try to get the default binding for this id
            let default_settings = settings::get_default_settings();
            match default_settings.bindings.get(&id) {
                Some(default_binding) => {
                    warn!(
                        "Binding '{}' not found in settings, creating from defaults",
                        id
                    );
                    default_binding.clone()
                }
                None => {
                    let error_msg = format!("Binding with id '{}' not found in defaults", id);
                    warn!("change_binding error: {}", error_msg);
                    return Ok(BindingResponse {
                        success: false,
                        binding: None,
                        error: Some(error_msg),
                    });
                }
            }
        }
    };
    // If this is the cancel binding, just update the settings and return
    // It's managed dynamically, so we don't register/unregister here
    if id == "cancel" {
        if let Some(mut b) = settings.bindings.get(&id).cloned() {
            b.current_binding = binding;
            settings.bindings.insert(id.clone(), b.clone());
            settings::write_settings(&app, settings);
            return Ok(BindingResponse {
                success: true,
                binding: Some(b.clone()),
                error: None,
            });
        }
    }

    // Check against reserved system shortcuts (macOS only for now)
    // Check against reserved system shortcuts
    if let Err(reason) = reserved::check_reserved_shortcut(&binding) {
            warn!("change_binding reserved shortcut error: {} - {}", binding, reason);
            return Ok(BindingResponse {
                success: false,
                binding: None,
                error: Some(reason),
            });
    }

    // If this is an fn-based shortcut (fn alone or fn+key), just update settings
    // fn key is handled by native fn_key_monitor.rs, not Tauri global shortcuts
    let binding_lower = binding.to_lowercase();
    if binding_lower == "fn" || binding_lower.starts_with("fn+") || binding_lower.contains("+fn") {
        // Block reserved system shortcuts - these are intercepted by macOS
        // and cannot be used as app shortcuts
        // Reserved shortcuts for fn are now handled by the global check above

        // Check for duplicates: see if any OTHER binding already uses this shortcut
        for (other_id, other_binding) in &settings.bindings {
            if other_id != &id && other_binding.current_binding.to_lowercase() == binding_lower {
                let error_msg = format!("Shortcut '{}' is already in use", binding);
                warn!("change_binding duplicate error for fn shortcut: {}", error_msg);
                return Ok(BindingResponse {
                    success: false,
                    binding: None,
                    error: Some(error_msg),
                });
            }
        }

        if let Some(mut b) = settings.bindings.get(&id).cloned() {
            // Unregister any existing non-fn shortcut first
            if let Err(e) = unregister_shortcut(&app, b.clone()) {
                debug!("No existing shortcut to unregister: {}", e);
            }
            b.current_binding = binding;
            settings.bindings.insert(id.clone(), b.clone());
            
            settings::write_settings(&app, settings);
            return Ok(BindingResponse {
                success: true,
                binding: Some(b.clone()),
                error: None,
            });
        }
    }

    // Unregister the existing binding
    if let Err(e) = unregister_shortcut(&app, binding_to_modify.clone()) {
        let error_msg = format!("Failed to unregister shortcut: {}", e);
        error!("change_binding error: {}", error_msg);
    }

    // Block reserved system shortcuts for macOS (non-fn shortcuts)


    // Validate the new shortcut before we touch the current registration
    if let Err(e) = validate_shortcut_string(&binding) {
        warn!("change_binding validation error: {}", e);
        return Err(e);
    }

    // Create an updated binding
    let mut updated_binding = binding_to_modify;
    updated_binding.current_binding = binding.clone();

    // Register the new binding
    if let Err(e) = register_shortcut(&app, updated_binding.clone()) {
        let error_msg = format!("Failed to register shortcut: {}", e);
        error!("change_binding error: {}", error_msg);
        return Ok(BindingResponse {
            success: false,
            binding: None,
            error: Some(error_msg),
        });
    }

    // Update the binding in the settings
    settings.bindings.insert(id.clone(), updated_binding.clone());

    // Save the settings
    settings::write_settings(&app, settings);

    // Return the updated binding
    Ok(BindingResponse {
        success: true,
        binding: Some(updated_binding),
        error: None,
    })
}

#[tauri::command]
#[specta::specta]
pub fn reset_binding(app: AppHandle, id: String) -> Result<BindingResponse, String> {
    // Get the default binding from the code-defined defaults (not from persisted settings)
    // This ensures that when defaults change in the code, "Reset to default" uses the new defaults
    let default_settings = settings::get_default_settings();
    let default_binding = default_settings
        .bindings
        .get(&id)
        .ok_or_else(|| format!("No default binding found for id '{}'", id))?;

    // Also update the stored default_binding field to keep it in sync with the code
    let mut current_settings = settings::get_settings(&app);
    if let Some(stored_binding) = current_settings.bindings.get_mut(&id) {
        stored_binding.default_binding = default_binding.default_binding.clone();
        settings::write_settings(&app, current_settings);
    }

    // Now change to the default binding
    change_binding(app, id, default_binding.default_binding.clone())
}

/// Reset multiple bindings atomically to their defaults.
/// This bypasses duplicate checking between the bindings being reset,
/// which solves the issue where resetting A then B fails if A's default
/// conflicts with B's current value.
#[tauri::command]
#[specta::specta]
pub fn reset_bindings(app: AppHandle, ids: Vec<String>) -> Result<Vec<BindingResponse>, String> {
    let default_settings = settings::get_default_settings();
    let mut current_settings = settings::get_settings(&app);
    let mut responses = Vec::new();

    // First pass: collect all the defaults and update settings atomically
    for id in &ids {
        let default_binding = default_settings
            .bindings
            .get(id)
            .ok_or_else(|| format!("No default binding found for id '{}'", id))?;

        if let Some(stored_binding) = current_settings.bindings.get_mut(id) {
            // Unregister any existing non-fn shortcut
            if let Err(e) = unregister_shortcut(&app, stored_binding.clone()) {
                debug!("No existing shortcut to unregister for {}: {}", id, e);
            }

            // Update to the default value
            stored_binding.default_binding = default_binding.default_binding.clone();
            stored_binding.current_binding = default_binding.default_binding.clone();

            responses.push(BindingResponse {
                success: true,
                binding: Some(stored_binding.clone()),
                error: None,
            });
        } else {
            responses.push(BindingResponse {
                success: false,
                binding: None,
                error: Some(format!("Binding '{}' not found", id)),
            });
        }
    }

    // Save all changes at once
    settings::write_settings(&app, current_settings.clone());

    // Second pass: register the reset shortcuts (skip fn-based, they use fn_key_monitor.rs)
    for response in &responses {
        if let Some(binding) = &response.binding {
            let binding_lower = binding.current_binding.to_lowercase();
            if binding_lower == "fn"
                || binding_lower.starts_with("fn+")
                || binding_lower.contains("+fn")
            {
                debug!(
                    "[reset_bindings] Skipping fn-based shortcut '{}' for {}",
                    binding.current_binding, binding.id
                );
                continue;
            }

            if let Err(e) = register_shortcut(&app, binding.clone()) {
                // Log but don't fail - the binding is already saved
                error!(
                    "[reset_bindings] Failed to register shortcut '{}' for {}: {}",
                    binding.current_binding, binding.id, e
                );
            } else {
                debug!(
                    "[reset_bindings] Registered shortcut '{}' for {}",
                    binding.current_binding, binding.id
                );
            }
        }
    }

    // Log what we did
    debug!("[reset_bindings] Reset {} bindings atomically", ids.len());

    Ok(responses)
}

#[tauri::command]
#[specta::specta]
pub fn change_audio_feedback_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_audio_feedback_volume_setting(app: AppHandle, volume: f32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback_volume = volume;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_sound_theme_setting(app: AppHandle, theme: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match theme.as_str() {
        "marimba" => SoundTheme::Marimba,
        "pop" => SoundTheme::Pop,
        "custom" => SoundTheme::Custom,
        other => {
            warn!("Invalid sound theme '{}', defaulting to marimba", other);
            SoundTheme::Marimba
        }
    };
    settings.sound_theme = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_translate_to_english_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.translate_to_english = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_selected_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.selected_language = language;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_overlay_position_setting(app: AppHandle, position: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match position.as_str() {
        "none" => OverlayPosition::None,
        "top" => OverlayPosition::Top,
        "bottom" => OverlayPosition::Bottom,
        other => {
            warn!("Invalid overlay position '{}', defaulting to bottom", other);
            OverlayPosition::Bottom
        }
    };
    settings.overlay_position = parsed;
    settings::write_settings(&app, settings);

    // Update overlay position without recreating window
    crate::utils::update_overlay_position(&app);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_debug_mode_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.debug_mode = enabled;
    settings::write_settings(&app, settings);

    // Emit event to notify frontend of debug mode change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "debug_mode",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_start_hidden_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.start_hidden = enabled;
    settings::write_settings(&app, settings);

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "start_hidden",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_autostart_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.autostart_enabled = enabled;
    settings::write_settings(&app, settings);

    // Apply the autostart setting immediately
    let autostart_manager = app.autolaunch();
    if enabled {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "autostart_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_update_checks_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.update_checks_enabled = enabled;
    settings::write_settings(&app, settings);

    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "update_checks_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn update_custom_words(app: AppHandle, words: Vec<CustomWordEntry>) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.dictionary = words;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_word_correction_threshold_setting(
    app: AppHandle,
    threshold: f64,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.word_correction_threshold = threshold;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_paste_method_setting(app: AppHandle, method: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match method.as_str() {
        "ctrl_v" => PasteMethod::CtrlV,
        "direct" => PasteMethod::Direct,
        "none" => PasteMethod::None,
        "shift_insert" => PasteMethod::ShiftInsert,
        "ctrl_shift_v" => PasteMethod::CtrlShiftV,
        other => {
            warn!("Invalid paste method '{}', defaulting to ctrl_v", other);
            PasteMethod::CtrlV
        }
    };
    settings.paste_method = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_clipboard_handling_setting(app: AppHandle, handling: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match handling.as_str() {
        "dont_modify" => ClipboardHandling::DontModify,
        "copy_to_clipboard" => ClipboardHandling::CopyToClipboard,
        other => {
            warn!(
                "Invalid clipboard handling '{}', defaulting to dont_modify",
                other
            );
            ClipboardHandling::DontModify
        }
    };
    settings.clipboard_handling = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_auto_submit_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.auto_submit = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_auto_submit_key_setting(app: AppHandle, key: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match key.as_str() {
        "enter" => AutoSubmitKey::Enter,
        "ctrl_enter" => AutoSubmitKey::CtrlEnter,
        "cmd_enter" => AutoSubmitKey::CmdEnter,
        other => {
            warn!("Invalid auto submit key '{}', defaulting to enter", other);
            AutoSubmitKey::Enter
        }
    };
    settings.auto_submit_key = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.post_process_enabled = enabled;
    settings::write_settings(&app, settings.clone());

    // Register or unregister the post-processing shortcut
    if let Some(binding) = settings
        .bindings
        .get("transcribe_with_post_process")
        .cloned()
    {
        if enabled {
            let _ = register_shortcut(&app, binding);
        } else {
            let _ = unregister_shortcut(&app, binding);
        }
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_base_url_setting(
    app: AppHandle,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let label = settings
        .post_process_provider(&provider_id)
        .map(|provider| provider.label.clone())
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let provider = settings
        .post_process_provider_mut(&provider_id)
        .expect("Provider looked up above must exist");

    if provider.id != "custom" {
        return Err(format!(
            "Provider '{}' does not allow editing the base URL",
            label
        ));
    }

    provider.base_url = base_url;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Generic helper to validate provider exists
fn validate_provider_exists(
    settings: &settings::AppSettings,
    provider_id: &str,
) -> Result<(), String> {
    if !settings
        .post_process_providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("Provider '{}' not found", provider_id));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_api_keys.insert(provider_id, api_key);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_model_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_models.insert(provider_id, model);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_provider_id = provider_id;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn add_post_process_prompt(
    app: AppHandle,
    name: String,
    prompt: String,
) -> Result<LLMPrompt, String> {
    let mut settings = settings::get_settings(&app);

    // Generate unique ID using timestamp and random component
    let id = format!("prompt_{}", chrono::Utc::now().timestamp_millis());

    let new_prompt = LLMPrompt {
        id: id.clone(),
        name,
        prompt,
    };

    settings.post_process_prompts.push(new_prompt.clone());
    settings::write_settings(&app, settings);

    Ok(new_prompt)
}

#[tauri::command]
#[specta::specta]
pub fn update_post_process_prompt(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    if let Some(existing_prompt) = settings
        .post_process_prompts
        .iter_mut()
        .find(|p| p.id == id)
    {
        existing_prompt.name = name;
        existing_prompt.prompt = prompt;
        settings::write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Prompt with id '{}' not found", id))
    }
}

#[tauri::command]
#[specta::specta]
pub fn delete_post_process_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Don't allow deleting the last prompt
    if settings.post_process_prompts.len() <= 1 {
        return Err("Cannot delete the last prompt".to_string());
    }

    // Find and remove the prompt
    let original_len = settings.post_process_prompts.len();
    settings.post_process_prompts.retain(|p| p.id != id);

    if settings.post_process_prompts.len() == original_len {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    // If the deleted prompt was selected, select the first one or None
    if settings.post_process_selected_prompt_id.as_ref() == Some(&id) {
        settings.post_process_selected_prompt_id =
            settings.post_process_prompts.first().map(|p| p.id.clone());
    }

    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_post_process_models(
    app: AppHandle,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let settings = settings::get_settings(&app);

    // Find the provider
    let provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            return Ok(vec![APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_string()]);
        }

        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            return Err("Apple Intelligence is only available on Apple silicon Macs running macOS 15 or later.".to_string());
        }
    }

    // Get API key
    let api_key = settings
        .post_process_api_keys
        .get(&provider_id)
        .cloned()
        .unwrap_or_default();

    // Skip fetching if no API key for providers that typically need one
    if api_key.trim().is_empty() && provider.id != "custom" {
        return Err(format!(
            "API key is required for {}. Please add an API key to list available models.",
            provider.label
        ));
    }

    crate::llm_client::fetch_models(provider, api_key).await
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_selected_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Verify the prompt exists
    if !settings.post_process_prompts.iter().any(|p| p.id == id) {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    settings.post_process_selected_prompt_id = Some(id);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_mute_while_recording_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.mute_while_recording = enabled;
    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_append_trailing_space_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.append_trailing_space = enabled;
    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_filler_word_filter_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.enable_filler_word_filter = enabled;
    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_hallucination_filter_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.enable_hallucination_filter = enabled;
    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_app_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.app_language = language.clone();
    settings::write_settings(&app, settings);

    // Refresh the tray menu with the new language (spawn async since Idle needs history check)
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tray::update_tray_menu_async(&app_clone, &tray::TrayIconState::Idle, Some(&language)).await;
    });

    Ok(())
}



/// Validate that a shortcut contains at least one non-modifier key.
/// The tauri-plugin-global-shortcut library requires at least one main key.
fn validate_shortcut_string(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Shortcut cannot be empty".into());
    }

    let modifiers = [
        "ctrl", "control", "shift", "alt", "option", "meta", "command", "cmd", "super", "win",
        "windows",
    ];
    let has_non_modifier = raw
        .split('+')
        .any(|part| !modifiers.contains(&part.trim().to_lowercase().as_str()));

    if has_non_modifier {
        Ok(())
    } else {
        Err("Shortcut must include a main key (letter, number, F-key, etc.) in addition to modifiers".into())
    }
}

/// Temporarily unregister a binding while the user is editing it in the UI.
/// This avoids firing the action while keys are being recorded.
#[tauri::command]
#[specta::specta]
pub fn suspend_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        // Skip fn-based shortcuts - they are handled by fn_key_monitor.rs, not global shortcuts
        let binding_lower = b.current_binding.to_lowercase();
        if binding_lower == "fn"
            || binding_lower.starts_with("fn+")
            || binding_lower.contains("+fn")
        {
            debug!(
                "suspend_binding: skipping fn-based shortcut '{}'",
                b.current_binding
            );
            return Ok(());
        }

        if let Err(e) = unregister_shortcut(&app, b) {
            error!("suspend_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

/// Re-register the binding after the user has finished editing.
#[tauri::command]
#[specta::specta]
pub fn resume_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        // Skip fn-based shortcuts - they are handled by fn_key_monitor.rs, not global shortcuts
        let binding_lower = b.current_binding.to_lowercase();
        if binding_lower == "fn"
            || binding_lower.starts_with("fn+")
            || binding_lower.contains("+fn")
        {
            debug!(
                "resume_binding: skipping fn-based shortcut '{}'",
                b.current_binding
            );
            return Ok(());
        }

        // Check if already registered (idempotency)
        if let Ok(shortcut) = b.current_binding.parse::<Shortcut>() {
            if app.global_shortcut().is_registered(shortcut) {
                debug!(
                    "resume_binding: shortcut '{}' already registered, skipping",
                    b.current_binding
                );
                return Ok(());
            }
        }

        if let Err(e) = register_shortcut(&app, b) {
            error!("resume_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

pub fn register_cancel_shortcut(app: &AppHandle) {
    // Cancel shortcut is disabled on Linux due to instability with dynamic shortcut registration
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        return;
    }

    #[cfg(not(target_os = "linux"))]
    {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(cancel_binding) = get_settings(&app_clone).bindings.get("cancel").cloned() {
                debug!("Attempting to register cancel shortcut: {}", cancel_binding.current_binding);
                match register_shortcut(&app_clone, cancel_binding) {
                    Ok(_) => debug!("Successfully registered cancel shortcut"),
                    Err(e) => error!("Failed to register cancel shortcut: {}", e),
                }
            } else {
                 warn!("No 'cancel' binding found in settings");
            }
        });
    }
}

pub fn unregister_cancel_shortcut(app: &AppHandle) {
    // Cancel shortcut is disabled on Linux due to instability with dynamic shortcut registration
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        return;
    }

    #[cfg(not(target_os = "linux"))]
    {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            // Check if we should actually unregister
            // If recording is active or any transcription session is active, we must KEEP the shortcut registered
            let audio_manager = app_clone.state::<Arc<AudioRecordingManager>>();
            let tm = app_clone.state::<Arc<TranscriptionManager>>();
            
            let is_recording = audio_manager.is_recording();
            // We need a thread-safe way to check if any session is active. 
            // Checking active_session_id is a reasonable proxy for "is transcribing"
            // assuming it's managed correctly.
            let is_transcribing = tm.is_any_session_active();

            if is_recording || is_transcribing {
                 info!(
                     "Skipping unregister_cancel_shortcut: session still active (recording={}, transcribing={})",
                     is_recording, is_transcribing
                 );
                 return;
            }

            if let Some(cancel_binding) = get_settings(&app_clone).bindings.get("cancel").cloned() {
                debug!("Unregistering cancel shortcut");
                // We ignore errors here as it might already be unregistered
                let _ = unregister_shortcut(&app_clone, cancel_binding);
            }
        });
    }
}

pub fn register_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    // Validate human-level rules first
    if let Err(e) = validate_shortcut_string(&binding.current_binding) {
        warn!(
            "_register_shortcut validation error for binding '{}': {}",
            binding.current_binding, e
        );
        return Err(e);
    }

    // Parse shortcut and return error if it fails
    let shortcut = match binding.current_binding.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Failed to parse shortcut '{}': {}",
                binding.current_binding, e
            );
            error!("_register_shortcut parse error: {}", error_msg);
            return Err(error_msg);
        }
    };

    // Prevent duplicate registrations that would silently shadow one another
    if app.global_shortcut().is_registered(shortcut) {
        let error_msg = format!("Shortcut '{}' is already in use", binding.current_binding);
        warn!("_register_shortcut duplicate error: {}", error_msg);
        return Err(error_msg);
    }

    // Clone binding.id for use in the closure
    let binding_id_for_closure = binding.id.clone();

    app.global_shortcut()
        .on_shortcut(shortcut, move |ah, scut, event| {
            if scut == &shortcut {
                let shortcut_string = scut.into_string();


                if let Some(action) = ACTION_MAP.get(&binding_id_for_closure) {
                    debug!("Global Shortcut Event: id='{}' state={:?} shortcut='{}'", binding_id_for_closure, event.state, shortcut_string);
                    if binding_id_for_closure == "cancel" {
                        if event.state == ShortcutState::Pressed {
                            tracing::debug!("Shortcut: 'cancel' pressed - triggering action.start");
                            action.start(ah, &binding_id_for_closure, &shortcut_string);
                        }
                    } else if binding_id_for_closure == "paste_last_transcript" {
                        // Paste last transcript is a one-shot action - always trigger on press
                        if event.state == ShortcutState::Pressed {
                            tracing::debug!("Shortcut: 'paste_last_transcript' pressed - triggering action.start");
                            action.start(ah, &binding_id_for_closure, &shortcut_string);
                        }
                    } else if binding_id_for_closure == "transcribe" {
                        // Main transcribe shortcut is ALWAYS Push-to-Talk (Press=Start, Release=Stop)
                        if event.state == ShortcutState::Pressed {
                            action.start(ah, &binding_id_for_closure, &shortcut_string);
                        } else if event.state == ShortcutState::Released {
                            action.stop(ah, &binding_id_for_closure, &shortcut_string);
                        }
                    } else if binding_id_for_closure == "transcribe_handsfree" {
                        // Hands-free shortcut is ALWAYS Toggle (Press to start/stop)
                        // Ignore Release events
                        if event.state == ShortcutState::Pressed {
                            // Determine action and update state while holding the lock
                            let should_start: bool;
                            {
                                let toggle_state_manager = ah.state::<ManagedToggleState>();
                                let mut states = match toggle_state_manager.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => {
                                        error!("Toggle state mutex poisoned for '{}', recovering", binding_id_for_closure);
                                        poisoned.into_inner()
                                    }
                                };

                                let is_currently_active = states
                                    .active_toggles
                                    .entry(binding_id_for_closure.clone())
                                    .or_insert(false);

                                should_start = !*is_currently_active;
                                *is_currently_active = should_start;
                            } // Lock released here

                            if should_start {
                                action.start(ah, &binding_id_for_closure, &shortcut_string);
                            } else {
                                action.stop(ah, &binding_id_for_closure, &shortcut_string);
                            }
                        }
                    } else {
                        // Default fallback for other shortcuts (assume Toggle behavior)
                        if event.state == ShortcutState::Pressed {
                            let should_start: bool;
                            {
                                let toggle_state_manager = ah.state::<ManagedToggleState>();
                                let mut states = match toggle_state_manager.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => {
                                        error!("Toggle state mutex poisoned for '{}', recovering", binding_id_for_closure);
                                        poisoned.into_inner()
                                    }
                                };

                                let is_currently_active = states
                                    .active_toggles
                                    .entry(binding_id_for_closure.clone())
                                    .or_insert(false);

                                should_start = !*is_currently_active;
                                *is_currently_active = should_start;
                            }

                            if should_start {
                                action.start(ah, &binding_id_for_closure, &shortcut_string);
                            } else {
                                action.stop(ah, &binding_id_for_closure, &shortcut_string);
                            }
                        }
                    }
                } else {
                    warn!(
                        "No action defined in ACTION_MAP for shortcut ID '{}'. Shortcut: '{}', State: {:?}",
                        binding_id_for_closure, shortcut_string, event.state
                    );
                }
            }
        })
        .map_err(|e| {
            let error_msg = format!("Couldn't register shortcut '{}': {}", binding.current_binding, e);
            error!("_register_shortcut registration error: {}", error_msg);
            error_msg
        })?;

    Ok(())
}

pub fn unregister_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let shortcut = match binding.current_binding.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Failed to parse shortcut '{}' for unregistration: {}",
                binding.current_binding, e
            );
            error!("_unregister_shortcut parse error: {}", error_msg);
            return Err(error_msg);
        }
    };

    app.global_shortcut().unregister(shortcut).map_err(|e| {
        let error_msg = format!(
            "Failed to unregister shortcut '{}': {}",
            binding.current_binding, e
        );
        error!("_unregister_shortcut error: {}", error_msg);
        error_msg
    })?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_show_tray_icon_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.show_tray_icon = enabled;
    settings::write_settings(&app, settings);

    // Apply change immediately
    tray::set_tray_visibility(&app, enabled);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_show_unload_model_in_tray_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.show_unload_model_in_tray = enabled;
    settings::write_settings(&app, settings);

    // Refresh tray menu to show/hide the unload model item
    tray::update_tray_menu_sync(&app, &tray::TrayIconState::Idle, None);

    Ok(())
}
