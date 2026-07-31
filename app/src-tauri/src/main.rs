#![allow(unexpected_cfgs)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod desktop_pet_window;
mod tray;

use serde_json::Value;
use std::sync::Mutex as StdMutex;
use sysinfo::System;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;

use familiar_core::config::FamiliarConfig;
use familiar_core::event::AgentSource;
use familiar_core::event_bus::EventBus;
use familiar_core::logger::init_logger;
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
    let _guard = init_logger("/tmp", "familiar_tauri.log").unwrap();

    let event_bus = EventBus::new(100, 100);
    let config = load_config();
    let state_machine = StateMachine::new(
        event_bus.clone(),
        config.renderer.desktop_pet.celebration_secs,
    );
    let event_bus_for_server = event_bus.clone();
    let config_for_setup = config.clone();

    let mut sys = System::new_all();
    sys.refresh_all();

    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(state_machine.clone())
        .manage(StdMutex::new(sys))
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
                        if let Ok(listener) = tokio::net::UnixListener::bind(&socket_path) {
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
                                                if let Some(source) = val
                                                    .get("source_client")
                                                    .and_then(|s| s.as_str())
                                                {
                                                    if let Some(payload) = val.get("payload") {
                                                        let event_name = val
                                                            .get("hook_event_name")
                                                            .and_then(|s| s.as_str())
                                                            .unwrap_or("");
                                                        let parsed_event = if source
                                                            == "antigravity"
                                                        {
                                                            let hook = AntigravityHook::new();
                                                            hook.parse(event_name, payload)
                                                        } else {
                                                            let agent_source = match source {
                                                                "codex" => AgentSource::Codex,
                                                                "claude-code" => {
                                                                    AgentSource::ClaudeCode
                                                                }
                                                                other => AgentSource::Custom(
                                                                    other.to_string(),
                                                                ),
                                                            };
                                                            let adapter = CliAgentHookAdapter::new(
                                                                agent_source,
                                                            );
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
                        }
                    });
                }
            }

            // Enable TCP listener on all platforms as fallback
            if let Some(port) = config.hooks.tcp_port {
                let bus = event_bus_for_server.clone();
                tauri::async_runtime::spawn(async move {
                    let addr = format!("127.0.0.1:{}", port);
                    if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
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
                                                            "claude-code" => {
                                                                AgentSource::ClaudeCode
                                                            }
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
                    }
                });
            }

            let app_handle = app.handle().clone();
            let sm_for_emit = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
                loop {
                    interval.tick().await;
                    let full_state = sm_for_emit.get_state().await;

                    use tauri::Manager;
                    if let Some(settings_win) = app_handle.get_webview_window("settings") {
                        let _ = settings_win.emit("settings_state_changed", &full_state);
                    }

                    if let Some(main_win) = app_handle.get_webview_window("main") {
                        let current_config = load_config();
                        let hidden_set: std::collections::HashSet<String> = current_config
                            .sessions
                            .hidden_sessions
                            .into_iter()
                            .collect();
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
                        let _ = main_win.emit("state_changed", &filtered_state);
                    } else {
                        let _ = app_handle.emit("state_changed", &full_state);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|_window, _event| {
            // no window event handling needed for single window
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_config,
            commands::save_config,
            commands::get_system_stats,
            commands::get_active_sessions,
            commands::open_settings_window,
            commands::open_url,
            commands::get_hooks_status,
            commands::get_hook_payload,
            commands::inject_hook,
            commands::uninstall_hook,
            commands::get_config_content,
            commands::preview_inject_hook,
            commands::preview_uninstall_hook,
            commands::quit_app,
            drag_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
