use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn set_update_menu_text(app: AppHandle, text: String) -> Result<(), String> {
    if let Some(menu) = app.menu() {
        // We know the ID is "check_updates" from src-tauri/src/menu.rs
        if let Some(item) = menu.get("check_updates") {
            let item = item.as_menuitem().ok_or("Menu item is not a standard MenuItem")?;
            item.set_text(text).map_err(|e: tauri::Error| e.to_string())?;
        }
    }
    Ok(())
}
