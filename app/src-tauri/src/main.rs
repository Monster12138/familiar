#![allow(unexpected_cfgs)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod desktop_pet_window;
mod tray;

use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use sysinfo::System;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;

use commands::AppConfigState;
use familiar_core::config::FamiliarConfig;
use familiar_core::event::AgentSource;
use familiar_core::event_bus::EventBus;
use familiar_core::logger::{default_log_dir, init_logger};
use familiar_core::state_machine::StateMachine;
use familiar_hooks::adapter::CliAgentHookAdapter;
use familiar_hooks::antigravity::AntigravityHook;

fn load_config() -> FamiliarConfig {
    crate::commands::load_config_from_paths()
}

// single window architecture

#[tauri::command]
fn drag_main_window(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.start_dragging();
    }
}

fn main() {
    let _guard = init_logger(default_log_dir(), "familiar_tauri.log").unwrap();

    let event_bus = EventBus::new(100, 100);
    let config = load_config();
    let app_config_state = Arc::new(AppConfigState::new(config.clone()));
    let state_machine = StateMachine::new(
        event_bus.clone(),
        config.renderer.desktop_pet.celebration_secs,
        config.renderer.desktop_pet.sleep_timeout_secs,
    );
    let event_bus_for_server = event_bus.clone();
    let config_for_setup = config.clone();

    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut max_disk_total = 0;
    let mut max_disk_used = 0;
    for disk in disks.list() {
        if disk.total_space() > max_disk_total {
            max_disk_total = disk.total_space();
            max_disk_used = disk.total_space() - disk.available_space();
        }
    }
    let sys_stats_state = crate::commands::SystemStatsState {
        system: sys,
        disks,
        cached_disk_used: max_disk_used,
        cached_disk_total: max_disk_total,
        last_disk_refresh: Some(std::time::Instant::now()),
    };

    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    let app_config_state_for_setup = app_config_state.clone();

    builder
        .manage(state_machine.clone())
        .manage(app_config_state.clone())
        .manage(StdMutex::new(sys_stats_state))
        .manage(event_bus.clone())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create_tray(app)?;

            let sm = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                sm.start_processing().await;
            });

            let config = config_for_setup;

            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = desktop_pet_window::initialize(&window) {
                    tracing::warn!("failed to initialize desktop pet window: {error}");
                }
                if let Err(error) =
                    desktop_pet_window::apply_settings(&window, &config.renderer.desktop_pet)
                {
                    tracing::warn!("failed to apply desktop pet window settings: {error}");
                }
            }

            #[cfg(unix)]
            {
                if let Some(socket_path) = config.hooks.socket_path {
                    let bus = event_bus_for_server.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = std::fs::remove_file(&socket_path);
                        let listener = match tokio::net::UnixListener::bind(&socket_path) {
                            Ok(listener) => listener,
                            Err(error) => {
                                // Do not fail silently: without this listener
                                // hook reports from coding agents never arrive.
                                tracing::error!(
                                    "failed to bind hook UDS listener on {}: {}",
                                    socket_path,
                                    error
                                );
                                return;
                            }
                        };
                        println!("Listening on UDS: {}", socket_path);
                        tracing::info!(
                            "Familiar desktop app started, listening on UDS: {}",
                            socket_path
                        );
                        loop {
                            if let Ok((stream, _)) = listener.accept().await {
                                let bus_clone = bus.clone();
                                tokio::spawn(async move {
                                    let (reader, _) = tokio::io::split(stream);
                                    let mut reader = tokio::io::BufReader::new(reader);
                                    let mut line = String::new();
                                    while let Ok(bytes) = reader.read_line(&mut line).await {
                                        if bytes == 0 {
                                            break;
                                        }
                                        if let Ok(val) = serde_json::from_str::<Value>(&line) {
                                            if let Some(source) =
                                                val.get("source_client").and_then(|s| s.as_str())
                                            {
                                                if let Some(payload) = val.get("payload") {
                                                    let event_name = val
                                                        .get("hook_event_name")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("");
                                                    let parsed_event = if source == "antigravity" {
                                                        let hook = AntigravityHook::new();
                                                        hook.parse(event_name, payload)
                                                    } else {
                                                        let agent_source = match source {
                                                            "codex" => AgentSource::Codex,
                                                            "claude-code" => {
                                                                AgentSource::ClaudeCode
                                                            }
                                                            "qoder" => AgentSource::Qoder,
                                                            other => AgentSource::Custom(
                                                                other.to_string(),
                                                            ),
                                                        };
                                                        let adapter =
                                                            CliAgentHookAdapter::new(agent_source);
                                                        let mut full_payload = payload.clone();
                                                        if let Some(obj) =
                                                            full_payload.as_object_mut()
                                                        {
                                                            obj.insert(
                                                                "hook_event_name".to_string(),
                                                                Value::String(
                                                                    event_name.to_string(),
                                                                ),
                                                            );
                                                        }
                                                        adapter.parse_hook_input(&full_payload)
                                                    };
                                                    if let Ok(event) = parsed_event {
                                                        let _ = bus_clone.publish(event).await;
                                                    }
                                                }
                                            }
                                        }
                                        line.clear();
                                    }
                                });
                            }
                        }
                    });
                }
            }

            // Enable TCP listener on all platforms as fallback
            if let Some(port) = config.hooks.tcp_port {
                let bus = event_bus_for_server.clone();
                tauri::async_runtime::spawn(async move {
                    let addr = format!("127.0.0.1:{}", port);
                    let listener = match tokio::net::TcpListener::bind(&addr).await {
                        Ok(listener) => listener,
                        Err(error) => {
                            // Do not fail silently: without this listener hook
                            // reports from coding agents never arrive. On
                            // Windows the port may fall into a Hyper-V/WSL
                            // excluded port range and bind with EACCES.
                            tracing::error!(
                                "failed to bind hook TCP listener on {}: {}",
                                addr,
                                error
                            );
                            return;
                        }
                    };
                    println!("Listening on TCP: {}", addr);
                    tracing::info!("Familiar desktop app listening on TCP: {}", addr);
                    loop {
                        if let Ok((stream, _)) = listener.accept().await {
                            let bus_clone = bus.clone();
                            tokio::spawn(async move {
                                let (reader, _) = tokio::io::split(stream);
                                let mut reader = tokio::io::BufReader::new(reader);
                                let mut line = String::new();
                                while let Ok(bytes) = reader.read_line(&mut line).await {
                                    if bytes == 0 {
                                        break;
                                    }
                                    if let Ok(val) = serde_json::from_str::<Value>(&line) {
                                        if let Some(source) =
                                            val.get("source_client").and_then(|s| s.as_str())
                                        {
                                            if let Some(payload) = val.get("payload") {
                                                let event_name = val
                                                    .get("hook_event_name")
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("");
                                                let parsed_event = if source == "antigravity" {
                                                    let hook = AntigravityHook::new();
                                                    hook.parse(event_name, payload)
                                                } else {
                                                    let agent_source = match source {
                                                        "codex" => AgentSource::Codex,
                                                        "claude-code" => AgentSource::ClaudeCode,
                                                        "qoder" => AgentSource::Qoder,
                                                        other => {
                                                            AgentSource::Custom(other.to_string())
                                                        }
                                                    };
                                                    let adapter =
                                                        CliAgentHookAdapter::new(agent_source);
                                                    let mut full_payload = payload.clone();
                                                    if let Some(obj) = full_payload.as_object_mut()
                                                    {
                                                        obj.insert(
                                                            "hook_event_name".to_string(),
                                                            Value::String(event_name.to_string()),
                                                        );
                                                    }
                                                    adapter.parse_hook_input(&full_payload)
                                                };
                                                if let Ok(event) = parsed_event {
                                                    let _ = bus_clone.publish(event).await;
                                                }
                                            }
                                        }
                                    }
                                    line.clear();
                                }
                            });
                        }
                    }
                });
            }

            let app_handle = app.handle().clone();
            let sm_for_emit = state_machine.clone();
            let config_state_for_emit = app_config_state_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
                let mut last_main_state_rev = 0u64;
                let mut last_main_config_rev = 0u64;
                let mut last_settings_state_rev = 0u64;

                loop {
                    interval.tick().await;

                    let current_state_rev = sm_for_emit.revision();
                    let current_config_rev = config_state_for_emit
                        .revision
                        .load(std::sync::atomic::Ordering::SeqCst);

                    use tauri::Manager;

                    // Emit settings state only if settings window exists AND is visible AND state changed
                    if let Some(settings_win) = app_handle.get_webview_window("settings") {
                        if settings_win.is_visible().unwrap_or(false)
                            && current_state_rev != last_settings_state_rev
                        {
                            let full_state = sm_for_emit.get_state().await;
                            let _ = settings_win.emit("settings_state_changed", &full_state);
                            last_settings_state_rev = current_state_rev;
                        }
                    }

                    // Emit main state only if state revision or config revision changed
                    if current_state_rev != last_main_state_rev
                        || current_config_rev != last_main_config_rev
                    {
                        let full_state = sm_for_emit.get_state().await;
                        let hidden_set = config_state_for_emit
                            .hidden_sessions
                            .read()
                            .unwrap()
                            .clone();

                        let mut filtered_state = full_state.clone();
                        if !hidden_set.is_empty() {
                            filtered_state
                                .agents
                                .retain(|a| !hidden_set.contains(&a.id));
                            filtered_state.active_agent_count = filtered_state.agents.len();
                            filtered_state.agents_by_category.clear();
                            for agent in &filtered_state.agents {
                                filtered_state
                                    .agents_by_category
                                    .entry(agent.category.clone())
                                    .or_default()
                                    .push(agent.clone());
                            }
                            if filtered_state.active_agent_count == 0 {
                                filtered_state.mood = familiar_core::state::FamiliarMood::Sleepy;
                            } else if filtered_state
                                .agents
                                .iter()
                                .any(|a| a.status == familiar_core::state::AgentStatus::Working)
                            {
                                filtered_state.mood = familiar_core::state::FamiliarMood::Busy;
                            } else if filtered_state
                                .agents
                                .iter()
                                .any(|a| a.status == familiar_core::state::AgentStatus::Thinking)
                            {
                                filtered_state.mood = familiar_core::state::FamiliarMood::Thinking;
                            } else if filtered_state
                                .agents
                                .iter()
                                .any(|a| a.status == familiar_core::state::AgentStatus::Completed)
                            {
                                filtered_state.mood =
                                    familiar_core::state::FamiliarMood::Celebrating;
                            } else if filtered_state.agents.iter().any(|a| {
                                a.status == familiar_core::state::AgentStatus::WaitingInput
                            }) {
                                filtered_state.mood = familiar_core::state::FamiliarMood::Watching;
                            } else {
                                filtered_state.mood = familiar_core::state::FamiliarMood::Idle;
                            }
                        }

                        if let Some(main_win) = app_handle.get_webview_window("main") {
                            let _ = main_win.emit("state_changed", &filtered_state);
                        } else {
                            let _ = app_handle.emit("state_changed", &full_state);
                        }

                        last_main_state_rev = current_state_rev;
                        last_main_config_rev = current_config_rev;
                    }
                }
            });

            Ok(())
        })
        .on_window_event({
            use std::sync::{Arc, Mutex};
            use std::time::{Duration, Instant};
            use tauri::Manager;

            let pos_state = Arc::new(Mutex::new((None::<(i32, i32)>, Instant::now(), false)));
            let app_config_state_for_pos = app_config_state.clone();

            move |window, event| {
                if window.label() == "main" {
                    if let tauri::WindowEvent::Moved(pos) = event {
                        let mut lock = pos_state.lock().unwrap();
                        lock.0 = Some((pos.x, pos.y));
                        lock.1 = Instant::now();

                        if !lock.2 {
                            lock.2 = true;
                            let pos_state_clone = pos_state.clone();
                            let app_handle = window.app_handle().clone();
                            let app_config_state_clone = app_config_state_for_pos.clone();
                            tauri::async_runtime::spawn(async move {
                                loop {
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                    let (target_pos, should_save) = {
                                        let mut lock = pos_state_clone.lock().unwrap();
                                        if lock.1.elapsed() >= Duration::from_millis(500) {
                                            lock.2 = false;
                                            (lock.0, true)
                                        } else {
                                            (None, false)
                                        }
                                    };

                                    if should_save {
                                        if let Some((x, y)) = target_pos {
                                            let mut config = app_config_state_clone.get_config();
                                            config.renderer.desktop_pet.position =
                                                format!("{},{}", x, y);
                                            let _ = crate::commands::save_config_internal(
                                                &app_handle,
                                                &app_config_state_clone,
                                                config,
                                            );
                                        }
                                        break;
                                    }
                                }
                            });
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_platform,
            commands::get_config,
            commands::save_config,
            commands::get_system_stats,
            commands::get_active_sessions,
            commands::delete_session,
            commands::open_settings_window,
            commands::open_url,
            commands::open_login_items_settings,
            commands::get_hooks_status,
            commands::get_hook_payload,
            commands::get_hook_details,
            commands::inject_hook,
            commands::uninstall_hook,
            commands::get_config_content,
            commands::preview_inject_hook,
            commands::preview_uninstall_hook,
            commands::test_hook_point,
            commands::get_sprite_packs,
            commands::import_sprite_pack,
            commands::get_active_sprite_pack,
            commands::open_sprite_dir,
            commands::quit_app,
            drag_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
