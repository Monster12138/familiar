use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::hook_trait::AgentHook;
use familiar_core::event::{AgentCategory, AgentEvent};

#[derive(Debug)]
pub struct CodexHook {
    // fields will be added later
}

impl CodexHook {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AgentHook for CodexHook {
    fn name(&self) -> &str {
        "codex"
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
