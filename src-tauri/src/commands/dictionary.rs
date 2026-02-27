use tauri::AppHandle;

use crate::user_dictionary::{self, CustomWordEntry};

#[tauri::command]
#[specta::specta]
pub fn get_user_dictionary(app: AppHandle) -> Result<Vec<CustomWordEntry>, String> {
    Ok(user_dictionary::get_dictionary_snapshot(&app).as_ref().clone())
}

#[tauri::command]
#[specta::specta]
pub fn set_user_dictionary(
    app: AppHandle,
    entries: Vec<CustomWordEntry>,
) -> Result<(), String> {
    user_dictionary::set_dictionary_entries(&app, entries)
}
