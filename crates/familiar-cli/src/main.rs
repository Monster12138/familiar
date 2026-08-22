pub mod auth;
pub mod hook_reporter;
pub mod hooks;

use anyhow::Result;
use clap::{Parser, Subcommand};
use familiar_api::StateStreamState;
use familiar_core::config::FamiliarConfig;
use tokio::net::TcpListener;

use familiar_core::event_bus::EventBus;
use familiar_core::logger::{default_log_dir, init_logger};
use familiar_core::state_machine::StateMachine;
use std::path::{Path, PathBuf};
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
        /// Optional JSON payload that overrides reading stdin. Useful for
        /// manual testing from a terminal, e.g.
        /// `--stdin-json '{"prompt":"hi"}'` (works in cmd, PowerShell
        /// and sh when the JSON is single-quoted).
        #[arg(long)]
        stdin_json: Option<String>,
        /// Explicit configuration file used to resolve the Hook ingest endpoint.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    /// Manage Agent hook configuration on the current machine.
    Hooks {
        #[command(subcommand)]
        command: hooks::HooksCommand,
    },
    /// Initialize or inspect the persisted server authentication token.
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Status subcommand
    Status,
    /// Run the Familiar server without a desktop UI
    Serve {
        /// Explicit configuration file for this server process.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// Override `server.bind` for this invocation.
        #[arg(long, value_name = "HOST:PORT")]
        bind: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Hook {
            source,
            event,
            stdin_json,
            config,
        } => {
            if let Some(config) = config {
                std::env::set_var("FAMILIAR_CONFIG", config);
            }
            crate::hook_reporter::run(source, event, stdin_json.as_deref()).await?;
        }
        Commands::Hooks { command } => {
            crate::hooks::run(command).await?;
        }
        Commands::Auth { command } => {
            crate::auth::run(command)?;
        }
        Commands::Status => {
            println!("Running status subcommand");
        }
        Commands::Serve { config, bind } => {
            run_server(config.as_deref(), bind.as_deref()).await?;
        }
    }

    Ok(())
}

async fn run_server(config_path: Option<&Path>, bind_override: Option<&str>) -> Result<()> {
    let _guard = init_logger(default_log_dir(), "familiar_server.log")?;
    let config = load_config(config_path)?;
    let event_bus = EventBus::new(100, 100);
    let ingest_bus = event_bus.clone();
    let state_machine = StateMachine::with_event_map(
        event_bus,
        config.renderer.desktop_pet.celebration_secs,
        config.renderer.desktop_pet.sleep_timeout_secs,
        std::sync::Arc::new(std::sync::RwLock::new(
            config.renderer.desktop_pet.event_status_agent_map(),
        )),
    );
    state_machine.start_processing().await;

    if let Some(port) = config.hooks.tcp_port {
        let tcp_bus = ingest_bus.clone();
        tokio::spawn(async move {
            if let Err(error) = familiar_hooks::ingest::serve_tcp(tcp_bus, port).await {
                tracing::error!(%error, "hook TCP listener stopped");
            }
        });
    }
    #[cfg(unix)]
    if let Some(path) = config.hooks.socket_path.clone() {
        tokio::spawn(async move {
            if let Err(error) = familiar_hooks::ingest::serve_unix(ingest_bus, path).await {
                tracing::error!(%error, "hook UDS listener stopped");
            }
        });
    }

    let auth_token = if config.server.auth.enabled {
        if let Some(persisted) = crate::auth::resolve_token(&config)? {
            if persisted.generated {
                if let Some(path) = config.server.auth.token_file.as_deref() {
                    println!(
                        "Familiar auth token initialized at {path}; retrieve it with `familiar-cli auth show --config <path>`"
                    );
                }
            }
            Some(persisted.token)
        } else {
            anyhow::bail!(
                "server authentication is enabled but no token is available; configure server.auth.token_file and set server.auth.auto_generate = true for first-run initialization"
            );
        }
    } else {
        None
    };
    let state = StateStreamState::new(
        state_machine,
        config.server.state_stream,
        auth_token,
        env!("CARGO_PKG_VERSION"),
    );
    let app = familiar_api::create_router(state);
    let bind = bind_override
        .map(ToOwned::to_owned)
        .or(config.server.bind)
        .unwrap_or_else(|| "127.0.0.1:19528".to_string());
    let addr: std::net::SocketAddr = bind.parse()?;

    if config.server.tls.enabled {
        let cert = config.server.tls.cert_path.ok_or_else(|| {
            anyhow::anyhow!("server.tls.cert_path is required when TLS is enabled")
        })?;
        let key = config.server.tls.key_path.ok_or_else(|| {
            anyhow::anyhow!("server.tls.key_path is required when TLS is enabled")
        })?;
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
        println!("Familiar server listening on https://{addr}");
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
            }
        });
        axum_server::bind_rustls(addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        println!("Familiar server listening on http://{addr} (TLS disabled)");
        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
    }
    Ok(())
}

pub(crate) fn load_config(explicit_path: Option<&Path>) -> Result<FamiliarConfig> {
    if let Some(path) = explicit_path {
        return FamiliarConfig::load_from_file(path);
    }
    if let Ok(path) = std::env::var("FAMILIAR_CONFIG") {
        return FamiliarConfig::load_from_file(path);
    }
    for path in familiar_core::platform::user_config_file_candidates() {
        if path.exists() {
            return FamiliarConfig::load_from_file(path);
        }
    }
    Ok(FamiliarConfig::default())
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn serve_is_the_only_server_subcommand() {
        assert!(matches!(
            Cli::try_parse_from(["familiar-cli", "serve"])
                .unwrap()
                .command,
            Commands::Serve {
                config: None,
                bind: None
            }
        ));
        assert!(Cli::try_parse_from(["familiar-cli", "headless"]).is_err());
    }

    #[test]
    fn hooks_management_commands_parse() {
        assert!(Cli::try_parse_from(["familiar-cli", "hooks", "status", "--json"]).is_ok());
        assert!(Cli::try_parse_from([
            "familiar-cli",
            "hooks",
            "preview",
            "--agent",
            "codex",
            "--operation",
            "uninstall"
        ])
        .is_ok());
        // Clap accepts the empty selection so the command can provide the
        // actionable `--agent`/`--all` error in the normal execution path.
        assert!(Cli::try_parse_from(["familiar-cli", "hooks", "install"]).is_ok());
    }

    #[test]
    fn auth_commands_parse() {
        assert!(Cli::try_parse_from(["familiar-cli", "auth", "init"]).is_ok());
        assert!(Cli::try_parse_from([
            "familiar-cli",
            "auth",
            "show",
            "--config",
            "/etc/familiar/server.toml"
        ])
        .is_ok());
    }
}
