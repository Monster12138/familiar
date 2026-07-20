use familiar_core::config::FamiliarConfig;

#[tauri::command]
pub fn get_config() -> Result<FamiliarConfig, String> {
    // Attempt to load from standard workspace relative paths
    let paths = [
        "config/default.toml",
        "../../config/default.toml",
    ];
    for p in paths {
        if std::path::Path::new(p).exists() {
            return FamiliarConfig::load_from_file(p).map_err(|e| e.to_string());
        }
    }
    Err("Config file not found".to_string())
}

#[tauri::command]
pub fn save_config(app_handle: tauri::AppHandle, config: FamiliarConfig) -> Result<(), String> {
    let paths = [
        "config/default.toml",
        "../../config/default.toml",
    ];
    for p in paths {
        if std::path::Path::new(p).exists() {
            let res = config.save_to_file(std::path::Path::new(p)).map_err(|e| e.to_string());
            if res.is_ok() {
                use tauri::Emitter;
                let _ = app_handle.emit("config_changed", config);
            }
            return res;
        }
    }
    Err("Config file not found".to_string())
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub async fn open_settings_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(_window) = app_handle.get_webview_window("settings") {
        return Ok(());
    }

    let _ = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("Settings")
    .inner_size(800.0, 600.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    let _ = open::that(url);
    Ok(())
}
