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
            if let Ok(c) = FamiliarConfig::load_from_file(p) {
                return Ok(c);
            }
        }
    }
    Ok(FamiliarConfig::default())
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
                use tauri::Manager;
                if let Some(window) = app_handle.get_webview_window("main") {
                    let scale = config.renderer.desktop_pet.scale as f64;
                    let _ = window.set_size(tauri::LogicalSize::new(160.0 * scale, 160.0 * scale));
                    let _ = window.set_always_on_top(config.renderer.desktop_pet.always_on_top);
                }
                
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

use familiar_hooks::antigravity::AntigravityHook;
use familiar_hooks::claude_code::ClaudeCodeHook;
use familiar_hooks::codex::CodexHook;
use familiar_hooks::hook_trait::AgentHook;
use serde_json::json;

fn get_hook_by_name(agent: &str) -> Option<Box<dyn AgentHook>> {
    match agent {
        "antigravity" => Some(Box::new(AntigravityHook::new())),
        "claude-code" => Some(Box::new(ClaudeCodeHook::new())),
        "codex" => Some(Box::new(CodexHook::new())),
        _ => None,
    }
}

#[tauri::command]
pub fn get_hooks_status() -> Result<serde_json::Value, String> {
    let hooks: Vec<Box<dyn AgentHook>> = vec![
        Box::new(AntigravityHook::new()),
        Box::new(ClaudeCodeHook::new()),
        Box::new(CodexHook::new()),
    ];
    
    let mut status_map = serde_json::Map::new();
    for hook in hooks {
        let agent_name = hook.name().to_string();
        let status = json!({
            "injected": hook.is_injected(),
            "config_path": hook.config_path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
        });
        status_map.insert(agent_name, status);
    }
    
    Ok(serde_json::Value::Object(status_map))
}

#[tauri::command]
pub fn get_hook_payload(agent: &str) -> Result<serde_json::Value, String> {
    if let Some(hook) = get_hook_by_name(agent) {
        return Ok(hook.get_injection_payload().unwrap_or(json!({})));
    }
    Err("Unknown agent".into())
}

#[tauri::command]
pub fn inject_hook(agent: &str) -> Result<(), String> {
    if let Some(hook) = get_hook_by_name(agent) {
        return hook.inject().map_err(|e| e.to_string());
    }
    Err("Unknown agent".into())
}

#[tauri::command]
pub fn uninstall_hook(agent: &str) -> Result<(), String> {
    if let Some(hook) = get_hook_by_name(agent) {
        return hook.uninstall().map_err(|e| e.to_string());
    }
    Err("Unknown agent".into())
}

#[tauri::command]
pub fn get_config_content(agent: &str) -> Result<String, String> {
    if let Some(hook) = get_hook_by_name(agent) {
        if let Some(path) = hook.config_path() {
            if path.exists() {
                return std::fs::read_to_string(&path).map_err(|e| e.to_string());
            }
            return Ok(String::new());
        }
    }
    Err("Unknown agent or config path".into())
}

#[derive(serde::Serialize)]
pub struct DiffPreview {
    pub before: String,
    pub after: String,
}

#[tauri::command]
pub fn preview_inject_hook(agent: &str) -> Result<DiffPreview, String> {
    if let Some(hook) = get_hook_by_name(agent) {
        let (before, after) = hook.preview_inject().map_err(|e| e.to_string())?;
        return Ok(DiffPreview { before, after });
    }
    Err("Unknown agent".into())
}

#[tauri::command]
pub fn preview_uninstall_hook(agent: &str) -> Result<DiffPreview, String> {
    if let Some(hook) = get_hook_by_name(agent) {
        let (before, after) = hook.preview_uninstall().map_err(|e| e.to_string())?;
        return Ok(DiffPreview { before, after });
    }
    Err("Unknown agent".into())
}
