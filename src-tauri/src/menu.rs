use tauri::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Emitter,
};

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let app_name = app.package_info().name.clone();

    // App Menu
    let about_item = PredefinedMenuItem::about(app, None::<&str>, Some(AboutMetadata::default()))?;
    let check_updates_item = MenuItem::with_id(app, "check_updates", "Check for Updates...", true, None::<&str>)?;
    let services_item = PredefinedMenuItem::services(app, Some("Services"))?;
    let hide_item = PredefinedMenuItem::hide(app, None::<&str>)?;
    let hide_others_item = PredefinedMenuItem::hide_others(app, Some("Hide Others"))?;
    let show_all_item = PredefinedMenuItem::show_all(app, Some("Show All"))?;
    let quit_item = PredefinedMenuItem::quit(app, None::<&str>)?;

    let app_menu = Submenu::with_items(
        app,
        &app_name,
        true,
        &[
            &about_item,
            &PredefinedMenuItem::separator(app)?,
            &check_updates_item,
            &PredefinedMenuItem::separator(app)?,
            &services_item,
            &PredefinedMenuItem::separator(app)?,
            &hide_item,
            &hide_others_item,
            &show_all_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    // File Menu
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &PredefinedMenuItem::close_window(app, Some("Close Window"))?,
        ],
    )?;

    // Edit Menu
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("Undo"))?,
            &PredefinedMenuItem::redo(app, Some("Redo"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some("Cut"))?,
            &PredefinedMenuItem::copy(app, Some("Copy"))?,
            &PredefinedMenuItem::paste(app, Some("Paste"))?,
            &PredefinedMenuItem::select_all(app, Some("Select All"))?,
        ],
    )?;

    // Window Menu
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some("Minimize"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, Some("Enter Full Screen"))?,
        ],
    )?;

    let menu = Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu, &window_menu])?;

    app.set_menu(menu)?;

    // Handle menu events
    app.on_menu_event(move |app, event| {
        if event.id() == "check_updates" {
            // Re-use logic from lib.rs or just emit
            // lib.rs checked settings, but standard behavior "Check for updates" usually forces a check.
            let _ = app.emit("check-for-updates", ());
            // Show main window to see the feedback
            let _ = crate::commands::window::show_main_window(app.clone());
        }
    });

    Ok(())
}
