use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use familiar_hooks::manager::{self, SUPPORTED_AGENTS};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum HooksCommand {
    /// Show the injection status of all supported agents.
    Status(StatusArgs),
    /// Preview an injection or uninstall without changing the config file.
    Preview(PreviewArgs),
    /// Install Familiar hooks for one agent or every supported agent.
    Install(AgentSelectionArgs),
    /// Remove Familiar hooks for one agent or every supported agent.
    Uninstall(AgentSelectionArgs),
    /// Dispatch a safe synthetic event through the configured hook endpoint.
    Test(TestArgs),
}

#[derive(Args, Debug, Clone)]
pub struct StatusArgs {
    /// Print machine-readable JSON instead of a human-readable table.
    #[arg(long)]
    pub json: bool,
    /// Configuration file used by the hook reporter for test operations.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct PreviewArgs {
    #[arg(long)]
    pub agent: String,
    /// Preview `inject` (default) or `uninstall`.
    #[arg(long, default_value = "inject", value_parser = ["inject", "uninstall"])]
    pub operation: String,
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct AgentSelectionArgs {
    /// Agent identifier, for example `claude-code` or `codex`.
    #[arg(long, conflicts_with = "all")]
    pub agent: Option<String>,
    /// Apply the operation to every supported agent.
    #[arg(long)]
    pub all: bool,
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct TestArgs {
    #[arg(long)]
    pub agent: String,
    #[arg(long)]
    pub event: String,
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct PreviewOutput {
    agent: String,
    operation: String,
    config_path: String,
    before: String,
    after: String,
}

pub async fn run(command: &HooksCommand) -> Result<()> {
    match command {
        HooksCommand::Status(args) => {
            set_config_env(args.config.as_ref());
            run_status(args)
        }
        HooksCommand::Preview(args) => {
            set_config_env(args.config.as_ref());
            run_preview(args)
        }
        HooksCommand::Install(args) => {
            set_config_env(args.config.as_ref());
            run_install(args)
        }
        HooksCommand::Uninstall(args) => {
            set_config_env(args.config.as_ref());
            run_uninstall(args)
        }
        HooksCommand::Test(args) => run_test(args).await,
    }
}

fn set_config_env(config: Option<&PathBuf>) {
    if let Some(config) = config {
        std::env::set_var("FAMILIAR_CONFIG", config);
        std::env::set_var("FAMILIAR_HOOK_CONFIG", config);
    } else if std::env::var_os("FAMILIAR_HOOK_CONFIG").is_none() {
        // A desktop process may already select its config through
        // FAMILIAR_CONFIG. Preserve that selection in injected commands too.
        if let Some(config) = std::env::var_os("FAMILIAR_CONFIG") {
            std::env::set_var("FAMILIAR_HOOK_CONFIG", config);
        }
    }
}

fn run_status(args: &StatusArgs) -> Result<()> {
    let statuses = manager::statuses();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&statuses)?);
    } else {
        for (agent, status) in statuses {
            let state = if status.injected {
                "injected"
            } else {
                "not injected"
            };
            println!("{agent}: {state} ({})", status.config_path);
        }
    }
    Ok(())
}

fn run_preview(args: &PreviewArgs) -> Result<()> {
    let hook = manager::hook_by_name(&args.agent)?;
    let (before, after) = match args.operation.as_str() {
        "inject" => hook.preview_inject()?,
        "uninstall" => hook.preview_uninstall()?,
        operation => return Err(anyhow!("unsupported preview operation: {operation}")),
    };
    let output = PreviewOutput {
        agent: args.agent.clone(),
        operation: args.operation.clone(),
        config_path: hook
            .config_path()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
        before,
        after,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn selected_agents(args: &AgentSelectionArgs) -> Result<Vec<&'static str>> {
    if args.all {
        return Ok(SUPPORTED_AGENTS.to_vec());
    }
    let agent = args
        .agent
        .as_deref()
        .ok_or_else(|| anyhow!("provide --agent <name> or --all"))?;
    manager::hook_by_name(agent)?;
    Ok(vec![SUPPORTED_AGENTS
        .iter()
        .copied()
        .find(|candidate| *candidate == agent)
        .ok_or_else(|| anyhow!("unknown agent: {agent}"))?])
}

fn run_install(args: &AgentSelectionArgs) -> Result<()> {
    let agents = selected_agents(args)?;
    for agent in agents {
        manager::hook_by_name(agent)?.inject()?;
        println!("installed hooks for {agent}");
    }
    Ok(())
}

fn run_uninstall(args: &AgentSelectionArgs) -> Result<()> {
    let agents = selected_agents(args)?;
    for agent in agents {
        manager::hook_by_name(agent)?.uninstall()?;
        println!("uninstalled hooks for {agent}");
    }
    Ok(())
}

async fn run_test(args: &TestArgs) -> Result<()> {
    set_config_env(args.config.as_ref());
    let hook = manager::hook_by_name(&args.agent)?;
    if !hook.is_injected() {
        return Err(anyhow!(
            "hooks for '{}' are not installed; run `familiar-cli hooks install --agent {}` first",
            args.agent,
            args.agent
        ));
    }

    let payload = json!({
        "hook_event_name": args.event,
        "session_id": format!("familiar-cli-test-{}", std::process::id()),
        "prompt": "[Familiar Hook Test]",
    });
    crate::hook_reporter::run(&args.agent, &args.event, Some(&payload.to_string())).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{selected_agents, AgentSelectionArgs};

    #[test]
    fn all_selects_every_supported_agent() {
        let args = AgentSelectionArgs {
            agent: None,
            all: true,
            config: None,
        };
        assert_eq!(selected_agents(&args).unwrap().len(), 5);
    }

    #[test]
    fn selection_requires_agent_or_all() {
        let args = AgentSelectionArgs {
            agent: None,
            all: false,
            config: None,
        };
        assert!(selected_agents(&args).is_err());
    }
}
