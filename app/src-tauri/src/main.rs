#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod tray;

use serde_json::Value;
use tauri::Emitter;
use tokio::io::AsyncBufReadExt;

use familiar_core::event_bus::EventBus;
use familiar_core::logger::init_logger;
use familiar_core::state_machine::StateMachine;
use familiar_core::config::FamiliarConfig;
use familiar_hooks::antigravity::AntigravityHook;

fn load_config() -> FamiliarConfig {
    let paths = [
        "config/default.toml",
        "../../config/default.toml",
    ];
    for p in paths {
        if std::path::Path::new(p).exists() {
            if let Ok(c) = FamiliarConfig::load_from_file(p) {
                return c;
            }
        }
    }
    FamiliarConfig::default()
}

fn main() {
    let _guard = init_logger("/tmp", "familiar_tauri.log").unwrap();

    let event_bus = EventBus::new(100, 100);
    let state_machine = StateMachine::new(event_bus.clone());

    let event_bus_for_server = event_bus.clone();

    tauri::Builder::default()
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            #[cfg(target_os = "macos")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let window_clone = window.clone();
                        let _ = window.run_on_main_thread(move || {
                            use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
                            use cocoa::base::id;
                            use objc::runtime::{Class, Object};
                            use objc::{msg_send, sel, sel_impl};
                            
                            #[link(name = "objc", kind = "dylib")]
                            extern "C" {
                                fn object_setClass(obj: *mut Object, cls: *const Class) -> *const Class;
                            }
                            
                            if let Ok(ns_win_ptr) = window_clone.ns_window() {
                                let ns_win = ns_win_ptr as id;
                                unsafe {
                                    if let Some(panel_class) = Class::get("NSPanel") {
                                        object_setClass(ns_win as *mut Object, panel_class);
                                    }
                                    
                                    let mask: cocoa::foundation::NSUInteger = msg_send![ns_win, styleMask];
                                    let _: () = msg_send![ns_win, setStyleMask: mask | 128];
                                    
                                    let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
                                    ns_win.setCollectionBehavior_(behavior);
                                    
                                    ns_win.setLevel_(26);
                                }
                            }
                        });
                    });
                }
            }

            tray::create_tray(app)?;

            let sm = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                sm.start_processing().await;
            });

            let config = load_config();

            #[cfg(unix)]
            {
                if let Some(socket_path) = config.hooks.socket_path {
                    let bus = event_bus_for_server.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = std::fs::remove_file(&socket_path);
                        if let Ok(listener) = tokio::net::UnixListener::bind(&socket_path) {
                            println!("Listening on UDS: {}", socket_path);
                            loop {
                                if let Ok((stream, _)) = listener.accept().await {
                                    let bus_clone = bus.clone();
                                    tokio::spawn(async move {
                                        let (reader, _) = tokio::io::split(stream);
                                        let mut reader = tokio::io::BufReader::new(reader);
                                        let mut line = String::new();
                                        while let Ok(bytes) = reader.read_line(&mut line).await {
                                            if bytes == 0 { break; }
                                            if let Ok(val) = serde_json::from_str::<Value>(&line) {
                                                if let Some(source) = val.get("source_client").and_then(|s| s.as_str()) {
                                                    if source == "antigravity" {
                                                        if let Some(payload) = val.get("payload") {
                                                            let event_name = val.get("hook_event_name").and_then(|s| s.as_str()).unwrap_or("");
                                                            let hook = AntigravityHook::new();
                                                            if let Ok(event) = hook.parse(event_name, payload) {
                                                                let _ = bus_clone.publish(event).await;
                                                            }
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

            #[cfg(windows)]
            {
                if let Some(port) = config.hooks.tcp_port {
                    let bus = event_bus_for_server.clone();
                    tauri::async_runtime::spawn(async move {
                        let addr = format!("127.0.0.1:{}", port);
                        if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
                            println!("Listening on TCP: {}", addr);
                            loop {
                                if let Ok((mut stream, _)) = listener.accept().await {
                                    let bus_clone = bus.clone();
                                    tokio::spawn(async move {
                                        let (reader, _) = tokio::io::split(stream);
                                        let mut reader = tokio::io::BufReader::new(reader);
                                        let mut line = String::new();
                                        while let Ok(bytes) = reader.read_line(&mut line).await {
                                            if bytes == 0 { break; }
                                            if let Ok(val) = serde_json::from_str::<Value>(&line) {
                                                if let Some(source) = val.get("source_client").and_then(|s| s.as_str()) {
                                                    if source == "antigravity" {
                                                        if let Some(payload) = val.get("payload") {
                                                            let event_name = val.get("hook_event_name").and_then(|s| s.as_str()).unwrap_or("");
                                                            let hook = AntigravityHook::new("".to_string());
                                                            if let Ok(event) = hook.parse(event_name, payload) {
                                                                let _ = bus_clone.publish(event).await;
                                                            }
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

            let app_handle = app.handle().clone();
            let sm_for_emit = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
                loop {
                    interval.tick().await;
                    let state = sm_for_emit.get_state().await;
                    let _ = app_handle.emit("state_changed", state);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_config,
            commands::save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
