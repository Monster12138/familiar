use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use familiar_core::event::{AgentEvent, AgentCategory};
use crate::hook_trait::AgentHook;

#[derive(Debug)]
pub struct ClaudeCodeHook {
    // fields will be added later
}

impl ClaudeCodeHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AgentHook for ClaudeCodeHook {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn category(&self) -> AgentCategory {
        AgentCategory::Coding
    }

    async fn start(&self, _sender: mpsc::Sender<AgentEvent>) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}
