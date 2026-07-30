pub mod hook_reporter;

use anyhow::Result;
use axum::{extract::State, routing::post, Json, Router};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;

use familiar_core::event_bus::EventBus;
use familiar_core::logger::init_logger;
use familiar_core::state_machine::StateMachine;
use familiar_hooks::antigravity::AntigravityHook;
// use familiar_hooks::hook_trait::AgentHook; removed

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Hook subcommand
    Hook {
        #[arg(long, default_value = "unknown")]
        source: String,
        #[arg(long, default_value = "unknown")]
        event: String,
    },
    /// Status subcommand
    Status,
    /// Headless subcommand
    Headless,
}

#[derive(Clone)]
struct AppState {
    event_bus: EventBus,
    antigravity_hook: Arc<AntigravityHook>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Hook { source, event } => {
            crate::hook_reporter::run(source, event).await?;
        }
        Commands::Status => {
            println!("Running status subcommand");
        }
        Commands::Headless => {
            println!("Starting Familiar headless daemon...");

            // Initialize logger
            let _guard = init_logger("/tmp", "familiar_daemon.log")?;

            // Setup core
            let event_bus = EventBus::new(100, 100);
            let state_machine = StateMachine::new(event_bus.clone(), 4);
            state_machine.start_processing().await;

            let state = AppState {
                event_bus,
                antigravity_hook: Arc::new(AntigravityHook::new()),
            };

            let app = Router::new()
                .route("/api/v1/notify", post(notify_handler))
                .with_state(state);

            let listener = TcpListener::bind("127.0.0.1:9528").await?;
            println!("Listening on 127.0.0.1:9528");
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}

async fn notify_handler(State(state): State<AppState>, Json(mut payload): Json<Value>) -> String {
    let source = payload["source_client"].as_str().unwrap_or("").to_string();
    let event_name = payload["hook_event_name"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if let Some(obj) = payload["payload"].as_object_mut() {
        obj.insert("hook_event_name".to_string(), Value::String(event_name.clone()));
    }

    let data = &payload["payload"];

    if source == "antigravity" {
        if let Ok(event) = state.antigravity_hook.parse(&event_name, data) {
            let _ = state.event_bus.publish(event).await;
            return "OK".into();
        }
    }

    "Ignored or Failed".into()
}
