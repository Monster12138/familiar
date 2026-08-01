use familiar_core::config::FamiliarConfig;
use familiar_core::state::AgentState;
use familiar_core::state_machine::StateMachine;
use std::sync::Mutex as StdMutex;
use sysinfo::{Disks, System};

#[tauri::command]
pub async fn get_active_sessions(
    sm: tauri::State<'_, StateMachine>,
) -> Result<Vec<AgentState>, String> {
    let state = sm.get_state().await;
    Ok(state.agents)
}

#[derive(Clone, serde::Serialize)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
}

#[tauri::command]
pub fn get_system_stats(sys_state: tauri::State<'_, StdMutex<System>>) -> SystemStats {
    let mut sys = sys_state.lock().unwrap();
    // Refresh only needed components
    sys.refresh_cpu_all();
    sys.refresh_memory();

    // Slight sleep to allow CPU calculation if needed, but normally we just rely on periodic frontend polling
    // sysinfo needs two data points to calculate CPU usage. Polling every 2s gives a good delta.

    let cpu_usage = sys.global_cpu_usage();
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();

    // Disks
    let disks = Disks::new_with_refreshed_list();
    let mut max_disk_total = 0;
    let mut max_disk_used = 0;

    for disk in disks.list() {
        if disk.total_space() > max_disk_total {
            max_disk_total = disk.total_space();
            max_disk_used = disk.total_space() - disk.available_space();
        }
    }

    let disk_total = max_disk_total;
    let disk_used = max_disk_used;

    SystemStats {
        cpu_usage,
        memory_used,
        memory_total,
        disk_used,
        disk_total,
    }
}

pub fn get_config_search_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".config").join("familiar").join("config.toml"));
    }
    paths.push(std::path::PathBuf::from("config/default.toml"));
    paths.push(std::path::PathBuf::from("../../config/default.toml"));
    paths
}

pub fn load_config_from_paths() -> FamiliarConfig {
    for p in get_config_search_paths() {
        if p.exists() {
            if let Ok(c) = FamiliarConfig::load_from_file(&p) {
                return c;
            }
        }
    }
    FamiliarConfig::default()
}

#[tauri::command]
pub fn get_config() -> Result<FamiliarConfig, String> {
    Ok(load_config_from_paths())
}

fn apply_and_emit_config(
    app_handle: &tauri::AppHandle,
    config: &FamiliarConfig,
) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app_handle.get_webview_window("main") {
        crate::desktop_pet_window::apply_settings(&window, &config.renderer.desktop_pet)?;
    }

    use tauri::Emitter;
    let _ = app_handle.emit("config_changed", config);
    Ok(())
}

#[tauri::command]
pub fn save_config(app_handle: tauri::AppHandle, config: FamiliarConfig) -> Result<(), String> {
    let search_paths = get_config_search_paths();
    for p in &search_paths {
        if p.exists() {
            let res = config.save_to_file(p).map_err(|e| e.to_string());
            if res.is_ok() {
                apply_and_emit_config(&app_handle, &config)?;
            }
            return res;
        }
    }

    // Save to user configuration directory (~/.config/familiar/config.toml) if no existing file was found
    if let Some(user_path) = search_paths.first() {
        if let Some(parent) = user_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let res = config.save_to_file(user_path).map_err(|e| e.to_string());
        if res.is_ok() {
            apply_and_emit_config(&app_handle, &config)?;
        }
        return res;
    }

    Err("Config file path resolution error".to_string())
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
pub async fn open_settings_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app_handle.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        // Hack to force window to front on macOS when app is in background
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
        let _ = window.set_focus();
        return Ok(());
    }
    let _ = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("Familiar Settings")
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

use familiar_core::sprite_pack::{SpritePackInfo, SpritePackManager};

use tauri::Manager;

#[tauri::command]
pub fn get_sprite_packs(app_handle: tauri::AppHandle) -> Result<Vec<SpritePackInfo>, String> {
    let resource_dir = app_handle.path().resource_dir().ok();
    Ok(SpritePackManager::discover_packs_with_extra(
        resource_dir.as_deref(),
    ))
}

#[tauri::command]
pub fn import_sprite_pack(path: String) -> Result<SpritePackInfo, String> {
    SpritePackManager::import_pack(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_sprite_pack(app_handle: tauri::AppHandle) -> Result<SpritePackInfo, String> {
    let config = load_config_from_paths();
    let active_id = config.renderer.desktop_pet.sprite;

    let resource_dir = app_handle.path().resource_dir().ok();
    let packs = SpritePackManager::discover_packs_with_extra(resource_dir.as_deref());
    if let Some(pack) = packs.iter().find(|p| p.manifest.id == active_id) {
        return Ok(pack.clone());
    }

    if let Some(pack) = packs.iter().find(|p| p.manifest.id == "default-cat") {
        return Ok(pack.clone());
    }

    if let Some(pack) = packs.into_iter().next() {
        return Ok(pack);
    }

    Err(format!("Sprite pack '{}' not found", active_id))
}
