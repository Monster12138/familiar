use familiar_core::cleanup::{
    bytes_of, delete_files, familiar_log_files_in, filter_by_age, DataCleanupSummary,
};
use familiar_core::config::{FamiliarConfig, RuntimeMode};
use familiar_core::event_bus::EventBus;
use familiar_core::logger::default_log_dir;
use familiar_core::state::{AgentState, EventStatusMap};
use familiar_core::state_machine::StateMachine;
use familiar_hooks::cleanup::{backup_dirs, scan_backups_in};
use std::collections::HashSet;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use sysinfo::{Disks, System};

pub struct AppConfigState {
    pub config: RwLock<FamiliarConfig>,
    pub hidden_sessions: RwLock<HashSet<String>>,
    /// Shared with `StateMachine` so event→status mapping changes take effect
    /// immediately on save (mirrors how `hidden_sessions` is kept in sync).
    pub event_status_map: Arc<RwLock<EventStatusMap>>,
    pub revision: AtomicU64,
}

impl AppConfigState {
    pub fn new(config: FamiliarConfig, event_status_map: Arc<RwLock<EventStatusMap>>) -> Self {
        let hidden_sessions = config.sessions.hidden_sessions.iter().cloned().collect();
        let map = config.renderer.desktop_pet.event_status_agent_map();
        if let Ok(mut guard) = event_status_map.write() {
            *guard = map;
        }
        Self {
            config: RwLock::new(config),
            hidden_sessions: RwLock::new(hidden_sessions),
            event_status_map,
            revision: AtomicU64::new(1),
        }
    }

    pub fn get_config(&self) -> FamiliarConfig {
        self.config.read().unwrap().clone()
    }

    pub fn update(&self, new_config: FamiliarConfig) {
        let new_hidden: HashSet<String> = new_config
            .sessions
            .hidden_sessions
            .iter()
            .cloned()
            .collect();
        let new_map = new_config.renderer.desktop_pet.event_status_agent_map();
        if let Ok(mut guard) = self.event_status_map.write() {
            *guard = new_map;
        }
        *self.config.write().unwrap() = new_config;
        *self.hidden_sessions.write().unwrap() = new_hidden;
        self.revision.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct SystemStatsState {
    pub system: System,
    pub disks: Disks,
    pub cached_disk_used: u64,
    pub cached_disk_total: u64,
    pub last_disk_refresh: Option<Instant>,
}

#[tauri::command]
pub async fn get_active_sessions(
    sm: tauri::State<'_, StateMachine>,
) -> Result<Vec<AgentState>, String> {
    let state = sm.get_state().await;
    Ok(state.agents)
}

#[tauri::command]
pub async fn delete_session(
    sm: tauri::State<'_, StateMachine>,
    agent_id: String,
) -> Result<bool, String> {
    Ok(sm.remove_agent(&agent_id).await)
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
pub fn get_system_stats(sys_state: tauri::State<'_, StdMutex<SystemStatsState>>) -> SystemStats {
    let mut stats = sys_state.lock().unwrap();
    // Refresh only needed components
    stats.system.refresh_cpu_all();
    stats.system.refresh_memory();

    let cpu_usage = stats.system.global_cpu_usage();
    let memory_used = stats.system.used_memory();
    let memory_total = stats.system.total_memory();

    // Refresh disk info at most once every 60 seconds
    let now = Instant::now();
    let should_refresh_disks = stats
        .last_disk_refresh
        .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(60));

    if should_refresh_disks {
        stats.disks.refresh();
        let mut max_disk_total = 0;
        let mut max_disk_used = 0;

        for disk in stats.disks.list() {
            if disk.total_space() > max_disk_total {
                max_disk_total = disk.total_space();
                max_disk_used = disk.total_space() - disk.available_space();
            }
        }

        stats.cached_disk_total = max_disk_total;
        stats.cached_disk_used = max_disk_used;
        stats.last_disk_refresh = Some(now);
    }

    SystemStats {
        cpu_usage,
        memory_used,
        memory_total,
        disk_used: stats.cached_disk_used,
        disk_total: stats.cached_disk_total,
    }
}

pub fn get_config_search_paths() -> Vec<std::path::PathBuf> {
    // Platform user config locations first (on Windows the platform config
    // dir, with the legacy ~/.config/familiar kept as fallback).
    let mut paths = familiar_core::platform::user_config_file_candidates();
    paths.push(std::path::PathBuf::from("config/default.toml"));
    paths.push(std::path::PathBuf::from("../../config/default.toml"));
    paths
}

// Candidate locations of the packaged default config shipped with the
// installer (next to the executable on Windows, inside the app bundle on
// macOS). Load-only: saving must never rewrite the bundled default file.
fn bundled_default_config_candidates() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("config/default.toml"));
            paths.push(dir.join("../Resources/config/default.toml"));
        }
    }
    paths
}

pub fn load_config_from_paths() -> FamiliarConfig {
    if let Ok(path) = std::env::var("FAMILIAR_CONFIG") {
        if let Ok(config) = FamiliarConfig::load_from_file(path) {
            return config;
        }
    }
    for p in get_config_search_paths() {
        if p.exists() {
            if let Ok(c) = FamiliarConfig::load_from_file(&p) {
                return c;
            }
        }
    }
    for p in bundled_default_config_candidates() {
        if p.exists() {
            if let Ok(c) = FamiliarConfig::load_from_file(&p) {
                return c;
            }
        }
    }
    FamiliarConfig::default()
}

#[tauri::command]
pub fn get_config(
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<FamiliarConfig, String> {
    Ok(config_state.get_config())
}

/// Scan (and optionally delete) familiar's hook backup files and log files,
/// per the `[cleanup]` config section. `dry_run = true` only counts and sums
/// eligible files so the UI can confirm before deletion.
#[tauri::command]
pub fn run_data_cleanup(
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    dry_run: bool,
) -> Result<DataCleanupSummary, String> {
    let cleanup = config_state.get_config().cleanup;

    let mut backups: Vec<std::path::PathBuf> = Vec::new();
    let mut logs: Vec<std::path::PathBuf> = Vec::new();
    if cleanup.backup_files {
        backups = filter_by_age(scan_backups_in(&backup_dirs()), cleanup.age_days);
    }
    if cleanup.log_files {
        logs = filter_by_age(familiar_log_files_in(&default_log_dir()), cleanup.age_days);
    }

    let mut summary = DataCleanupSummary {
        backup_count: backups.len(),
        log_count: logs.len(),
        freed_bytes: bytes_of(&backups) + bytes_of(&logs),
        failures: Vec::new(),
    };

    if !dry_run {
        let mut all = backups;
        all.append(&mut logs);
        let (freed, failures) = delete_files(&all);
        summary.freed_bytes = freed;
        summary.failures = failures;
    }
    Ok(summary)
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

pub fn save_config_internal(
    app_handle: &tauri::AppHandle,
    config_state: &Arc<AppConfigState>,
    config: FamiliarConfig,
) -> Result<(), String> {
    let search_paths = get_config_search_paths();
    for p in &search_paths {
        if p.exists() {
            let res = config.save_to_file(p).map_err(|e| e.to_string());
            if res.is_ok() {
                config_state.update(config.clone());
                apply_and_emit_config(app_handle, &config)?;
            }
            return res;
        }
    }

    // Save to the preferred user config location if no existing file was found
    if let Some(user_path) = search_paths.first() {
        if let Some(parent) = user_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let res = config.save_to_file(user_path).map_err(|e| e.to_string());
        if res.is_ok() {
            config_state.update(config.clone());
            apply_and_emit_config(app_handle, &config)?;
        }
        return res;
    }

    Err("Config file path resolution error".to_string())
}

#[tauri::command]
pub fn save_config(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    config: FamiliarConfig,
) -> Result<(), String> {
    save_config_internal(&app_handle, &config_state, config)
}

/// The compiled-in application version (sourced from the workspace
/// `Cargo.toml`), so the UI never hardcodes a version that goes stale.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check for a newer release. `force=true` always queries GitHub (manual
/// check / tray); `force=false` is the startup auto-check gated by
/// `check_on_startup` and the configured interval.
#[tauri::command]
pub async fn check_for_updates(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    pending: tauri::State<'_, Arc<crate::updates::PendingUpdateState>>,
    force: bool,
) -> Result<familiar_core::update::CheckUpdateResult, String> {
    crate::updates::run_check(&app_handle, &config_state, &pending, force).await
}

/// Take the most recent update result, if any. Consumed once so a settings
/// window created after a startup check still shows the prompt, without
/// re-prompting on every open.
#[tauri::command]
pub fn get_pending_update(
    pending: tauri::State<'_, Arc<crate::updates::PendingUpdateState>>,
) -> Option<familiar_core::update::CheckUpdateResult> {
    pending.0.write().unwrap().take()
}

/// Record a version the user chose to skip; the reminder is suppressed for
/// that version until a newer one is released.
#[tauri::command]
pub async fn skip_update(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    pending: tauri::State<'_, Arc<crate::updates::PendingUpdateState>>,
    version: String,
) -> Result<(), String> {
    let mut config = config_state.get_config();
    config.update.skipped_version = Some(version);
    *pending.0.write().unwrap() = None;
    crate::commands::save_config_internal(&app_handle, &config_state, config)
}

/// Permanently ignore a version so it never prompts again.
#[tauri::command]
pub async fn ignore_update(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    pending: tauri::State<'_, Arc<crate::updates::PendingUpdateState>>,
    version: String,
) -> Result<(), String> {
    let mut config = config_state.get_config();
    if !config.update.ignored_versions.contains(&version) {
        config.update.ignored_versions.push(version);
    }
    *pending.0.write().unwrap() = None;
    crate::commands::save_config_internal(&app_handle, &config_state, config)
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Current operating system (`std::env::consts::OS`), used by the frontend
/// to adapt settings UI that only applies on some platforms.
#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
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

/// Open (or focus) the first-run onboarding window, mirroring the settings
/// window lifecycle: focus it if it already exists, otherwise create it.
#[tauri::command]
pub async fn open_onboard_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app_handle.get_webview_window("onboard") {
        let _ = window.unminimize();
        let _ = window.show();
        // Force window to front on macOS when the app is in the background.
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
        let _ = window.set_focus();
        return Ok(());
    }
    let _ = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "onboard",
        tauri::WebviewUrl::App("onboard.html".into()),
    )
    .title("Familiar")
    .inner_size(760.0, 640.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Mark the first-run onboarding as completed (called when the user finishes
/// or skips the onboard page). Persists `general.onboarded = true`.
#[tauri::command]
pub async fn complete_onboarding(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<(), String> {
    let mut config = config_state.get_config();
    config.general.onboarded = true;
    crate::commands::save_config_internal(&app_handle, &config_state, config)
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    let _ = open::that(url);
    Ok(())
}

/// Open the platform's login-item / startup settings.
///
/// macOS has no public API to register login items, so deep-link into
/// System Settings. Windows has no equivalent GUI; the per-user Startup
/// folder is opened instead (a shortcut placed there starts at login).
/// Linux uses the freedesktop autostart directory.
#[tauri::command]
pub fn open_login_items_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        open::that("x-apple.systempreferences:com.apple.LoginItems-Settings.extension")
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .or_else(dirs::data_dir)
            .ok_or_else(|| "Cannot resolve APPDATA".to_string())?;
        let startup = appdata
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        std::fs::create_dir_all(&startup).map_err(|e| e.to_string())?;
        open::that(&startup).map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let autostart = dirs::config_dir()
            .ok_or_else(|| "Cannot resolve config dir".to_string())?
            .join("autostart");
        std::fs::create_dir_all(&autostart).map_err(|e| e.to_string())?;
        open::that(&autostart).map_err(|e| e.to_string())
    }
}

use familiar_hooks::hook_trait::AgentHook;
use familiar_hooks::manager;
use serde_json::json;

fn get_hook_by_name(agent: &str) -> Option<Box<dyn AgentHook>> {
    manager::hook_by_name(agent).ok()
}

fn ensure_local_hooks(config: &FamiliarConfig) -> Result<(), String> {
    if config.runtime.mode != RuntimeMode::Local {
        return Err(
            "Hooks can only be changed from the local runtime; run `familiar-cli hooks` on the remote server"
                .to_string(),
        );
    }
    Ok(())
}

fn run_local_hooks_cli(args: &[String]) -> Result<Vec<u8>, String> {
    let cli = familiar_hooks::bin_path::resolve_cli_bin_path();
    let output = std::process::Command::new(&cli)
        .args(args)
        .output()
        .map_err(|error| format!("failed to start {cli}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(if stderr.trim().is_empty() {
            format!("familiar-cli exited with {}", output.status)
        } else {
            stderr.trim().to_string()
        });
    }
    Ok(output.stdout)
}

async fn fetch_remote_hooks_status(
    config: &familiar_core::config::RemoteConfig,
) -> Result<serde_json::Value, String> {
    let endpoint = config
        .endpoint
        .as_deref()
        .ok_or_else(|| "remote.endpoint is required in remote mode".to_string())?;
    let scheme = if config.tls { "https" } else { "http" };
    let url = format!("{scheme}://{endpoint}/api/v1/hooks/status");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            config.connect_timeout_secs.max(1),
        ))
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    if let Some(token) = read_remote_token(config)? {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("remote Hooks status request failed: HTTP {status}"));
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())
}

/// Resolve the local credential file used by the remote desktop client.
/// Keeping this path separate from `config.toml` prevents config saves and
/// config previews from ever serializing the bearer token itself.
pub(crate) fn remote_token_file_path(
    config: &familiar_core::config::RemoteConfig,
) -> std::path::PathBuf {
    config
        .token_file
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| familiar_core::platform::user_config_dir().join("remote-token"))
}

pub(crate) fn read_remote_token(
    config: &familiar_core::config::RemoteConfig,
) -> Result<Option<String>, String> {
    let path = remote_token_file_path(config);
    let value = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "read remote token file {}: {error}",
                path.display()
            ))
        }
    };
    let token = value.trim().to_string();
    if token.is_empty() {
        return Err(format!("remote token file is empty: {}", path.display()));
    }
    Ok(Some(token))
}

fn write_remote_token(path: &std::path::Path, token: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create remote token directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid remote token file path: {}", path.display()))?;
    let temporary = path.with_file_name(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("get token file timestamp: {error}"))?
            .as_nanos()
    ));

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let write_result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("create temporary remote token file: {error}"))?;
        file.write_all(token.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|error| format!("write temporary remote token file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("flush temporary remote token file: {error}"))?;
        drop(file);
        if let Err(error) = std::fs::rename(&temporary, path) {
            #[cfg(windows)]
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                std::fs::remove_file(path)
                    .map_err(|remove_error| format!("replace remote token file: {remove_error}"))?;
                std::fs::rename(&temporary, path)
                    .map_err(|rename_error| format!("replace remote token file: {rename_error}"))?;
            } else {
                return Err(format!("replace remote token file: {error}"));
            }
            #[cfg(not(windows))]
            return Err(format!("replace remote token file: {error}"));
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write_result?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .map_err(|error| format!("read remote token permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(path, permissions)
            .map_err(|error| format!("restrict remote token permissions: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_remote_token(
    config_state: tauri::State<'_, Arc<AppConfigState>>,
    token: String,
) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("remote token cannot be empty".to_string());
    }
    let path = remote_token_file_path(&config_state.get_config().remote);
    write_remote_token(&path, token)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn get_hooks_status(
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<serde_json::Value, String> {
    let config = config_state.get_config();
    if config.runtime.mode == RuntimeMode::Remote {
        return fetch_remote_hooks_status(&config.remote).await;
    }
    let output = tokio::task::spawn_blocking(|| {
        run_local_hooks_cli(&[
            "hooks".to_string(),
            "status".to_string(),
            "--json".to_string(),
        ])
    })
    .await
    .map_err(|error| error.to_string())??;
    serde_json::from_slice(&output).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_hook_payload(agent: &str) -> Result<serde_json::Value, String> {
    if let Some(hook) = get_hook_by_name(agent) {
        return Ok(hook.get_injection_payload().unwrap_or(json!({})));
    }
    Err("Unknown agent".into())
}

#[tauri::command]
pub async fn inject_hook(
    agent: String,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<(), String> {
    let config = config_state.get_config();
    ensure_local_hooks(&config)?;
    tokio::task::spawn_blocking(move || {
        run_local_hooks_cli(&[
            "hooks".to_string(),
            "install".to_string(),
            "--agent".to_string(),
            agent,
        ])
        .map(|_| ())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn uninstall_hook(
    agent: String,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<(), String> {
    let config = config_state.get_config();
    ensure_local_hooks(&config)?;
    tokio::task::spawn_blocking(move || {
        run_local_hooks_cli(&[
            "hooks".to_string(),
            "uninstall".to_string(),
            "--agent".to_string(),
            agent,
        ])
        .map(|_| ())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_config_content(
    agent: &str,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<String, String> {
    ensure_local_hooks(&config_state.get_config())?;
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DiffPreview {
    pub before: String,
    pub after: String,
}

#[tauri::command]
pub async fn preview_inject_hook(
    agent: String,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<DiffPreview, String> {
    ensure_local_hooks(&config_state.get_config())?;
    let output = tokio::task::spawn_blocking(move || {
        run_local_hooks_cli(&[
            "hooks".to_string(),
            "preview".to_string(),
            "--agent".to_string(),
            agent,
        ])
    })
    .await
    .map_err(|error| error.to_string())??;
    serde_json::from_slice(&output).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn preview_uninstall_hook(
    agent: String,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<DiffPreview, String> {
    ensure_local_hooks(&config_state.get_config())?;
    let output = tokio::task::spawn_blocking(move || {
        run_local_hooks_cli(&[
            "hooks".to_string(),
            "preview".to_string(),
            "--agent".to_string(),
            agent,
            "--operation".to_string(),
            "uninstall".to_string(),
        ])
    })
    .await
    .map_err(|error| error.to_string())??;
    serde_json::from_slice(&output).map_err(|error| error.to_string())
}

// ---------------------------------------------------------------------------
// Hook details & test commands
// ---------------------------------------------------------------------------

#[derive(Clone, serde::Serialize)]
pub struct HookPointInfo {
    pub event_name: String,
    pub command: String,
    /// Full shell command including a mocked stdin payload, ready to be
    /// pasted into a terminal by the user (e.g. `echo <json> | <command>`).
    pub test_command: String,
    pub matcher: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct AgentHookDetail {
    pub agent: String,
    pub config_path: String,
    pub injected: bool,
    pub hook_points: Vec<HookPointInfo>,
}

/// Build a minimal mocked stdin payload for a hook event, matching the shape
/// that `CliAgentHookAdapter` / `AntigravityHook::parse` expect. Avoid
/// single quotes inside string values so the JSON stays safe inside the
/// shell-quoting of the generated test command.
fn mock_payload_for_event(event_name: &str) -> serde_json::Value {
    let base = serde_json::json!({
        "hook_event_name": event_name,
        "session_id": "manual-test",
    });

    match event_name {
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" => {
            let mut p = base;
            p["tool_name"] = serde_json::json!("Bash");
            p["tool_input"] = serde_json::json!({"command": "echo Familiar hook test"});
            p
        }
        "UserPromptSubmit" | "SessionStart" => {
            let mut p = base;
            p["prompt"] = serde_json::json!("[Familiar Test] Hook point test");
            p
        }
        _ => base,
    }
}

/// Build a copy-pasteable shell command for manual testing: appends
/// `--stdin-json '<json>'` to the hook command so the mocked payload is
/// passed as an argument instead of stdin. Single-quoting keeps the JSON
/// intact in cmd, PowerShell and sh alike (the CLI strips the quotes).
fn build_test_command(event_name: &str, command: &str) -> String {
    let json_str = mock_payload_for_event(event_name).to_string();
    format!("{} --stdin-json '{}'", command, json_str)
}

/// Extract hook-point details from an injection payload.
/// Payload shape:
///   { "hooks" | "familiar": { "EventName": [ { "hooks": [{"command":"..."}] } ] } }
fn extract_hook_points(payload: &serde_json::Value) -> Vec<HookPointInfo> {
    let mut points = Vec::new();

    let hooks_obj = payload
        .get("hooks")
        .or_else(|| payload.get("familiar"))
        .and_then(|v| v.as_object());

    let Some(hooks_obj) = hooks_obj else {
        return points;
    };

    for (event_name, event_array) in hooks_obj {
        let Some(items) = event_array.as_array() else {
            continue;
        };
        for item in items {
            let matcher = item
                .get("matcher")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Commands may be directly on the item (Antigravity style) or nested
            // inside a "hooks" array (Claude Code / Codex / Qoder style).
            let extract_cmd = |obj: &serde_json::Value| -> Option<String> {
                obj.get("command")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            };

            if let Some(cmd) = extract_cmd(item) {
                points.push(HookPointInfo {
                    event_name: event_name.clone(),
                    test_command: build_test_command(event_name, &cmd),
                    command: cmd,
                    matcher,
                });
            } else if let Some(inner_hooks) = item.get("hooks").and_then(|v| v.as_array()) {
                for inner in inner_hooks {
                    if let Some(cmd) = extract_cmd(inner) {
                        points.push(HookPointInfo {
                            event_name: event_name.clone(),
                            test_command: build_test_command(event_name, &cmd),
                            command: cmd,
                            matcher: matcher.clone(),
                        });
                    }
                }
            }
        }
    }

    points
}

#[tauri::command]
pub fn get_hook_details(
    agent: &str,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<AgentHookDetail, String> {
    ensure_local_hooks(&config_state.get_config())?;
    let hook = get_hook_by_name(agent).ok_or_else(|| "Unknown agent".to_string())?;
    let payload = hook
        .get_injection_payload()
        .unwrap_or(serde_json::json!({}));
    let hook_points = extract_hook_points(&payload);

    Ok(AgentHookDetail {
        agent: agent.to_string(),
        config_path: hook
            .config_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        injected: hook.is_injected(),
        hook_points,
    })
}

#[derive(serde::Serialize)]
pub struct TestHookResult {
    pub success: bool,
    pub message: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[tauri::command]
pub async fn test_hook_point(
    agent: &str,
    event_name: &str,
    mode: &str,
    event_bus: tauri::State<'_, EventBus>,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<TestHookResult, String> {
    ensure_local_hooks(&config_state.get_config())?;
    match mode {
        "event_bus" => test_via_event_bus(agent, event_name, &event_bus).await,
        "command" => test_via_shell(agent, event_name),
        _ => Err("Unknown mode, expected 'event_bus' or 'command'".into()),
    }
}

async fn test_via_event_bus(
    agent: &str,
    event_name: &str,
    event_bus: &EventBus,
) -> Result<TestHookResult, String> {
    use familiar_core::event::AgentSource;
    use familiar_hooks::adapter::CliAgentHookAdapter;
    use familiar_hooks::antigravity::AntigravityHook;

    // Simple unique session id without depending on external crates.
    let test_session_id = format!(
        "test-{}-{}-{:?}",
        agent,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    // Build a minimal test payload that the adapter can parse.
    let base_payload = serde_json::json!({
        "hook_event_name": event_name,
        "session_id": test_session_id,
    });

    let payload = match event_name {
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" => {
            let mut p = base_payload;
            p["tool_name"] = serde_json::json!("Bash");
            p["tool_input"] = serde_json::json!({"command": "echo Familiar hook test"});
            p
        }
        "UserPromptSubmit" | "SessionStart" => {
            let mut p = base_payload;
            p["prompt"] = serde_json::json!("[Familiar Test] Hook point test");
            p
        }
        _ => base_payload,
    };

    let parsed = if agent == "antigravity" {
        let hook = AntigravityHook::new();
        hook.parse(event_name, &payload).map_err(|e| e.to_string())
    } else {
        let source = match agent {
            "codex" => AgentSource::Codex,
            "claude-code" => AgentSource::ClaudeCode,
            "qoder" => AgentSource::Qoder,
            "deepseek-harness" => AgentSource::DeepSeekHarness,
            other => AgentSource::Custom(other.to_string()),
        };
        let adapter = CliAgentHookAdapter::new(source);
        adapter
            .parse_hook_input(&payload)
            .map_err(|e| e.to_string())
    }?;

    event_bus.publish(parsed).await.map_err(|e| e.to_string())?;

    Ok(TestHookResult {
        success: true,
        message: format!(
            "Test event '{}' for '{}' published via event bus",
            event_name, agent
        ),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
    })
}

fn test_via_shell(agent: &str, event_name: &str) -> Result<TestHookResult, String> {
    let hook = get_hook_by_name(agent).ok_or_else(|| "Unknown agent".to_string())?;
    let payload = hook
        .get_injection_payload()
        .unwrap_or(serde_json::json!({}));
    let hook_points = extract_hook_points(&payload);

    let target = hook_points
        .iter()
        .find(|p| p.event_name == event_name)
        .ok_or_else(|| {
            format!(
                "Event '{}' not found in injection payload for '{}'",
                event_name, agent
            )
        })?;

    // The command is stored as `"path/to/cli" hook --source ...` inside the
    // payload. Pass it through as-is: both `cmd /C` and `sh -c` strip the
    // surrounding quotes themselves, and trimming them here would leave an
    // unbalanced quote that breaks execution on Windows.
    let cmd_str = target.command.as_str();

    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", cmd_str])
            .output()
            .map_err(|e| format!("Failed to execute: {}", e))?
    } else {
        std::process::Command::new("sh")
            .args(["-c", cmd_str])
            .output()
            .map_err(|e| format!("Failed to execute: {}", e))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();
    let success = output.status.success();

    Ok(TestHookResult {
        success,
        message: if success {
            format!(
                "Command executed successfully (exit code: {})",
                exit_code.unwrap_or(-1)
            )
        } else {
            format!("Command failed (exit code: {})", exit_code.unwrap_or(-1))
        },
        stdout,
        stderr,
        exit_code,
    })
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
pub async fn import_sprite_pack(path: Option<String>) -> Result<SpritePackInfo, String> {
    let import_path = if let Some(p) = path.filter(|s| !s.is_empty()) {
        std::path::PathBuf::from(p)
    } else {
        let picked = rfd::AsyncFileDialog::new()
            .set_title("选择素材包 (.fpack / .zip)")
            .add_filter("Familiar Sprite Pack", &["fpack", "zip"])
            .pick_file()
            .await;

        match picked {
            Some(file) => file.path().to_path_buf(),
            None => return Err("Cancelled".to_string()),
        }
    };

    SpritePackManager::import_pack(&import_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_sprite_pack(
    app_handle: tauri::AppHandle,
    config_state: tauri::State<'_, Arc<AppConfigState>>,
) -> Result<SpritePackInfo, String> {
    let config = config_state.get_config();
    let active_id = config.renderer.desktop_pet.sprite;

    let resource_dir = app_handle.path().resource_dir().ok();
    let packs = SpritePackManager::discover_packs_with_extra(resource_dir.as_deref());
    if let Some(pack) = packs.iter().find(|p| p.manifest.id == active_id) {
        return Ok(pack.clone());
    }

    if let Some(pack) = packs.iter().find(|p| p.manifest.id == "tabby-cat") {
        return Ok(pack.clone());
    }

    if let Some(pack) = packs.into_iter().next() {
        return Ok(pack);
    }

    Err(format!("Sprite pack '{}' not found", active_id))
}

#[tauri::command]
pub fn open_sprite_dir() -> Result<(), String> {
    let dir = SpritePackManager::get_user_sprite_dir();
    let _ = std::fs::create_dir_all(&dir);
    open::that(&dir).map_err(|e| e.to_string())
}
