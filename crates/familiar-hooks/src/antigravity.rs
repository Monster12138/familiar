use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use familiar_core::event::{AgentEvent, AgentCategory};
use crate::hook_trait::AgentHook;
use crate::adapter::CliAgentHookAdapter;
use familiar_core::event::AgentSource;

#[derive(Debug)]
pub struct AntigravityHook {
    adapter: CliAgentHookAdapter,
}

impl AntigravityHook {
    pub fn new() -> Self {
        Self {
            adapter: CliAgentHookAdapter::new(AgentSource::Antigravity),
        }
    }
    
    pub fn parse(&self, json: &serde_json::Value) -> Result<AgentEvent> {
        self.adapter.parse_hook_input(json)
    }
}

#[async_trait]
impl AgentHook for AntigravityHook {
    fn name(&self) -> &str {
        "antigravity"
    }

    fn category(&self) -> AgentCategory {
        AgentCategory::Coding
    }

    async fn start(&self, _sender: mpsc::Sender<AgentEvent>) -> Result<()> {
        // The actual listening might happen via the CLI stdin or a named pipe.
        // This trait method is a placeholder for when we spawn actual daemons.
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}
