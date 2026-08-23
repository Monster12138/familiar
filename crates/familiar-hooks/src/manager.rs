//! Shared Hooks management facade used by the CLI, desktop shell, and server
//! status endpoint. The actual agent-specific merge/uninstall logic remains
//! implemented by each [`AgentHook`].

use crate::antigravity::AntigravityHook;
use crate::claude_code::ClaudeCodeHook;
use crate::codex::CodexHook;
use crate::deepseek_harness::DeepSeekHarnessHook;
use crate::hook_trait::AgentHook;
use crate::qoder::QoderHook;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::BTreeMap;

/// Agent identifiers supported by Hook injection and status reporting.
pub const SUPPORTED_AGENTS: &[&str] = &[
    "antigravity",
    "claude-code",
    "codex",
    "deepseek-harness",
    "qoder",
];

/// The intentionally small status payload shared by local CLI output and the
/// remote read-only API. The path is useful for local administration, while
/// the server API may redact it before returning the response.
#[derive(Debug, Clone, Serialize)]
pub struct HookStatus {
    pub injected: bool,
    pub config_path: String,
}

/// Construct the AgentHook implementation for a supported agent.
pub fn hook_by_name(agent: &str) -> Result<Box<dyn AgentHook>> {
    match agent {
        "antigravity" => Ok(Box::new(AntigravityHook::new())),
        "claude-code" => Ok(Box::new(ClaudeCodeHook::new())),
        "codex" => Ok(Box::new(CodexHook::new())),
        "deepseek-harness" => Ok(Box::new(DeepSeekHarnessHook::new())),
        "qoder" => Ok(Box::new(QoderHook::new())),
        _ => Err(anyhow!(
            "unknown agent '{agent}'; supported agents: {}",
            SUPPORTED_AGENTS.join(", ")
        )),
    }
}

/// Return status for every supported Agent in stable order.
pub fn statuses() -> BTreeMap<String, HookStatus> {
    SUPPORTED_AGENTS
        .iter()
        .filter_map(|agent| {
            let hook = hook_by_name(agent).ok()?;
            Some((
                (*agent).to_string(),
                HookStatus {
                    injected: hook.is_injected(),
                    config_path: hook
                        .config_path()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{hook_by_name, statuses, SUPPORTED_AGENTS};

    #[test]
    fn supported_agents_have_factories() {
        for agent in SUPPORTED_AGENTS {
            assert_eq!(hook_by_name(agent).unwrap().name(), *agent);
        }
    }

    #[test]
    fn unknown_agent_error_lists_supported_agents() {
        let error = match hook_by_name("not-an-agent") {
            Ok(_) => panic!("unknown agent should fail"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("supported agents"));
        assert!(error.contains("claude-code"));
    }

    #[test]
    fn status_contains_all_supported_agents() {
        let status = statuses();
        assert_eq!(status.len(), SUPPORTED_AGENTS.len());
        for agent in SUPPORTED_AGENTS {
            assert!(status.contains_key(*agent));
        }
    }
}
