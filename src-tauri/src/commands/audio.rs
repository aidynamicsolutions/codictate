use crate::audio_feedback;
use crate::audio_toolkit::audio::{list_input_devices, list_output_devices};
use crate::managers::audio::{AudioRecordingManager, MicrophoneMode};
use crate::settings::{get_settings, write_settings};
use tracing::warn;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Type)]
pub struct CustomSounds {
    start: bool,
    stop: bool,
}

fn custom_sound_exists(app: &AppHandle, sound_type: &str) -> bool {
    app.path()
        .resolve(
            format!("custom_{}.wav", sound_type),
            tauri::path::BaseDirectory::AppData,
        )
        .map_or(false, |path| path.exists())
}

#[tauri::command]
#[specta::specta]
pub fn check_custom_sounds(app: AppHandle) -> CustomSounds {
    CustomSounds {
        start: custom_sound_exists(&app, "start"),
        stop: custom_sound_exists(&app, "stop"),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct AudioDevice {
    pub index: String,
    pub name: String,
    pub is_default: bool,
    pub is_bluetooth: bool,
}

#[tauri::command]
#[specta::specta]
pub fn update_microphone_mode(app: AppHandle, always_on: bool) -> Result<(), String> {
    // Update settings
    let mut settings = get_settings(&app);
    settings.always_on_microphone = always_on;
    write_settings(&app, settings);

    // Update the audio manager mode
    let rm = app.state::<Arc<AudioRecordingManager>>();
    let new_mode = if always_on {
        MicrophoneMode::AlwaysOn
    } else {
        MicrophoneMode::OnDemand
    };

    rm.update_mode(new_mode)
        .map_err(|e| format!("Failed to update microphone mode: {}", e))
}

#[tauri::command]
#[specta::specta]
pub fn get_microphone_mode(app: AppHandle) -> Result<bool, String> {
    let settings = get_settings(&app);
    Ok(settings.always_on_microphone)
}

#[tauri::command]
#[specta::specta]
pub async fn get_available_microphones(app: AppHandle) -> Result<Vec<AudioDevice>, String> {
    // Run blocking device enumeration on a thread pool
    let devices = tauri::async_runtime::spawn_blocking(move || {
        list_input_devices().map_err(|e| format!("Failed to list audio devices: {}", e))
    }).await
    .map_err(|e| format!("Task join error: {}", e))??;

    let mut result = vec![AudioDevice {
        index: "default".to_string(),
        name: "Default".to_string(),
        is_default: true,
        is_bluetooth: false,
    }];

    let settings = get_settings(&app);
    let selected = settings.selected_microphone.as_deref().unwrap_or("default");

    result.extend(devices.into_iter().filter_map(|d| {
        let is_bt = crate::audio_device_info::is_device_bluetooth(&d.name);
        
        let is_virtual = crate::audio_device_info::is_device_virtual(&d.name);
        
        if is_virtual && d.name != selected {
            return None;
        }

        Some(AudioDevice {
            index: d.index,
            name: d.name,
            is_default: d.is_default,
            is_bluetooth: is_bt,
        })
    }));

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn set_selected_microphone(app: AppHandle, device_name: String) -> Result<(), String> {
    let rm = app.state::<Arc<AudioRecordingManager>>().inner().clone();
    
    // When user explicitly selects a device, clear the ENTIRE blocklist.
    // This prevents stale blocklist entries from accumulating across sessions
    // and blocking valid fallback devices like MacBook Pro.
    let was_blocked = {
        let blocked = rm.get_blocked_devices();
        let was_in_blocklist = blocked.contains(&device_name);
        
        if !blocked.is_empty() {
            tracing::info!(
                "User explicitly selected device '{}'. Clearing entire blocklist ({:?}) for fresh start.",
                device_name, blocked
            );
            rm.set_blocked_devices(std::collections::HashSet::new());
        }
        // Reset detection counters for fresh start
        rm.reset_dead_device_counters();
        
        was_in_blocklist
    };
    
    let mut settings = get_settings(&app);
    settings.selected_microphone = if device_name == "default" {
        None
    } else {
        Some(device_name.clone())
    };
    write_settings(&app, settings);

    // Update the audio manager to use the new device
    // Spawn this on a background task so we don't block the UI while waiting for the stream
    // to initialize (which can take 3-6s if devices are failing/retrying)
    tauri::async_runtime::spawn(async move {
        if let Err(e) = rm.update_selected_device() {
            tracing::error!("Failed to update selected device (background task): {}", e);
        }
        
        // If device was previously blocked, log that we're re-testing
        if was_blocked {
            tracing::info!("Re-testing previously blocked device '{}' per user request", device_name);
        }
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_selected_microphone(app: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app);
    Ok(settings
        .selected_microphone
        .unwrap_or_else(|| "default".to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn get_available_output_devices() -> Result<Vec<AudioDevice>, String> {
    let devices =
        list_output_devices().map_err(|e| format!("Failed to list output devices: {}", e))?;

    let mut result = vec![AudioDevice {
        index: "default".to_string(),
        name: "Default".to_string(),
        is_default: true,
        is_bluetooth: false,
    }];

    result.extend(devices.into_iter().map(|d| {
        let is_bt = crate::audio_device_info::is_device_bluetooth(&d.name);
        AudioDevice {
            index: d.index,
            name: d.name,
            is_default: d.is_default,
            is_bluetooth: is_bt,
        }
    }));

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn set_selected_output_device(app: AppHandle, device_name: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.selected_output_device = if device_name == "default" {
        None
    } else {
        Some(device_name)
    };
    write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_selected_output_device(app: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app);
    Ok(settings
        .selected_output_device
        .unwrap_or_else(|| "default".to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn play_test_sound(app: AppHandle, sound_type: String) {
    let sound = match sound_type.as_str() {
        "start" => audio_feedback::SoundType::Start,
        "stop" => audio_feedback::SoundType::Stop,
        _ => {
            warn!("Unknown sound type: {}", sound_type);
            return;
        }
    };
    audio_feedback::play_test_sound(&app, sound);
}

#[tauri::command]
#[specta::specta]
pub fn set_clamshell_microphone(app: AppHandle, device_name: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.clamshell_microphone = if device_name == "default" {
        None
    } else {
        Some(device_name)
    };
    write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_clamshell_microphone(app: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app);
    Ok(settings
        .clamshell_microphone
        .unwrap_or_else(|| "default".to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn is_recording(app: AppHandle) -> bool {
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    audio_manager.is_recording()
}

/// Start microphone preview mode - opens the mic stream to emit levels without recording
#[tauri::command]
#[specta::specta]
pub async fn start_mic_preview(app: AppHandle) -> Result<(), String> {
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    audio_manager
        .start_microphone_stream()
        .map_err(|e| format!("Failed to start mic preview: {}", e))
}

/// Stop microphone preview mode - closes the mic stream
#[tauri::command]
#[specta::specta]
pub fn stop_mic_preview(app: AppHandle) {
    let audio_manager = app.state::<Arc<AudioRecordingManager>>();
    // Only stop if not currently recording
    if !audio_manager.is_recording() {
        audio_manager.stop_microphone_stream();
    }
}
