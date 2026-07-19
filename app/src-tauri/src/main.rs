#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod tray;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            tray::create_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
