mod commands;
mod tabs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(tabs::TabState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_tabs,
            commands::new_tab,
            commands::close_tab,
            commands::switch_tab,
            commands::navigate,
            commands::go_back,
            commands::go_forward,
            commands::update_content_bounds,
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running flow");
}
