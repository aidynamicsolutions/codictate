use crate::settings;
use tauri::{AppHandle, Manager};

#[tauri::command]
#[specta::specta]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    // Check if we should start hidden based on settings
    // This command is called from frontend on init, so we double-check the setting
    // to ensure we don't accidentally show the window if it was meant to be hidden 
    // (e.g. if the user launched with --hidden flag or settings prefer it, 
    // although this command is primarily for the explicit "I am ready" signal).
    
    // However, the "start hidden" logic was likely handled in setup().
    // If we are here, the frontend is running, meaning the window is technically "created" but hidden.
    // If the user WANTS to start hidden, they still need the frontend to run for things like tray communication usually.
    // But if start_hidden is true, we should probably NOT show it here.
    
    let settings = settings::get_settings(&app);
    
    if settings.start_hidden {
        // If start_hidden is true, we generally expect the window to act as an accessory
        // and stay hidden until explicitly requested by user (e.g. clicking tray).
        // So we do nothing here.
        return Ok(());
    }

    if let Some(main_window) = app.get_webview_window("main") {
        // First, ensure the window is visible
        if let Err(e) = main_window.show() {
            tracing::error!("Failed to show window: {}", e);
            return Err(e.to_string());
        }
        // Then, bring it to the front and give it focus
        if let Err(e) = main_window.set_focus() {
            tracing::error!("Failed to focus window: {}", e);
            return Err(e.to_string());
        }
        
        // Optional: On macOS, ensure the app becomes active if it was an accessory
        #[cfg(target_os = "macos")]
        {
           let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        }
    } else {
        tracing::error!("Main window not found.");
        return Err("Main window not found".to_string());
    }
    
    Ok(())
}
