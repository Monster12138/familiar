use anyhow::Result;
use async_trait::async_trait;
use familiar_core::event::{AgentCategory, AgentEvent};
use tokio::sync::mpsc;

#[async_trait]
pub trait AgentHook: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> AgentCategory;
    async fn start(&self, sender: mpsc::Sender<AgentEvent>) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
