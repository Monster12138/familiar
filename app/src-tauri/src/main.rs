#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod tray;

use axum::{extract::State, routing::post, Json, Router};
use serde_json::Value;
use std::sync::Arc;
use tauri::Emitter;
use tokio::net::TcpListener;

use familiar_core::event_bus::EventBus;
use familiar_core::logger::init_logger;
use familiar_core::state_machine::StateMachine;
use familiar_hooks::antigravity::AntigravityHook;
use familiar_hooks::hook_trait::AgentHook;

#[derive(Clone)]
struct AppState {
    event_bus: EventBus,
    antigravity_hook: Arc<AntigravityHook>,
}

async fn notify_handler(State(state): State<AppState>, Json(mut payload): Json<Value>) -> String {
    let source = payload["source_client"].as_str().unwrap_or("").to_string();
    let event_name = payload["hook_event_name"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if let Some(obj) = payload["payload"].as_object_mut() {
        obj.insert("hook_event_name".to_string(), Value::String(event_name));
    }

    let data = &payload["payload"];
    if source == "antigravity" {
        if let Ok(event) = state.antigravity_hook.parse(data) {
            let _ = state.event_bus.publish(event).await;
            return "OK".into();
        }
    }
    "Ignored or Failed".into()
}

fn main() {
    // init logger
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
                                    // 1. Swizzle to NSPanel so macOS WindowServer treats it as a floating utility
                                    if let Some(panel_class) = Class::get("NSPanel") {
                                        object_setClass(ns_win as *mut Object, panel_class);
                                    }
                                    
                                    // 2. Add NSNonactivatingPanelMask (128) to the style mask
                                    let mask: cocoa::foundation::NSUInteger = msg_send![ns_win, styleMask];
                                    let _: () = msg_send![ns_win, setStyleMask: mask | 128];
                                    
                                    // 3. Set collection behavior for all spaces & fullscreen auxiliary
                                    let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
                                    ns_win.setCollectionBehavior_(behavior);
                                    
                                    // 4. Force floating window level
                                    ns_win.setLevel_(26); // NSMainMenuWindowLevel + 2
                                }
                            }
                        });
                    });
                }
            }

            // Setup tray
            tray::create_tray(app)?;

            // Start State Machine
            let sm = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                sm.start_processing().await;
            });

            // Start API Server and Hooks
            let transcript_path = "/Users/sam.gl/.gemini/antigravity/brain/dcaad95a-be83-4785-9636-bf935bf3676b/.system_generated/logs/transcript.jsonl".to_string();
            let antigravity_hook = Arc::new(AntigravityHook::new(transcript_path));
            
            // Actually start the hook so it tails the file
            let hook_clone = antigravity_hook.clone();
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);
            tauri::async_runtime::spawn(async move {
                let _ = hook_clone.start(tx).await;
            });
            
            // Bridge the hook's mpsc channel to our broadcast event bus
            let bus_for_hook = event_bus_for_server.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = bus_for_hook.publish(event).await;
                }
            });

            let app_state = AppState {
                event_bus: event_bus_for_server,
                antigravity_hook,
            };

            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .route("/api/v1/notify", post(notify_handler))
                    .with_state(app_state);

                if let Ok(listener) = TcpListener::bind("127.0.0.1:9528").await {
                    println!("Tauri Daemon Listening on 127.0.0.1:9528");
                    let _ = axum::serve(listener, router).await;
                }
            });

            // Listen to state changes and emit to webview
            let app_handle = app.handle().clone();
            let _bus_rx = event_bus.subscribe();
            // Wait, we need to poll StateMachine for RenderState, or have StateMachine emit RenderState events.
            // Actually, StateMachine `apply_event` currently just updates its internal state. 
            // The simplest way for MVP is to poll StateMachine periodically, or modify StateMachine to take a channel.
            // For now, let's poll StateMachine every 500ms and emit RenderState.
            let sm_for_emit = state_machine.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
                loop {
                    interval.tick().await;
                    let state = sm_for_emit.get_state().await;
                    // Emit to all windows
                    let _ = app_handle.emit("state_changed", state);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
