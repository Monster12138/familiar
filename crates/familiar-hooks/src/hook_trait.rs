use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use familiar_core::event::{AgentCategory, AgentEvent};

#[async_trait]
pub trait AgentHook: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> AgentCategory;
    async fn start(&self, sender: mpsc::Sender<AgentEvent>) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
