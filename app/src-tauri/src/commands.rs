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
pub fn save_config(config: FamiliarConfig) -> Result<(), String> {
    let paths = [
        "config/default.toml",
        "../../config/default.toml",
    ];
    for p in paths {
        if std::path::Path::new(p).exists() {
            return config.save_to_file(std::path::Path::new(p)).map_err(|e| e.to_string());
        }
    }
    Err("Config file not found".to_string())
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
