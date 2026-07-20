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
    
    // Hook injection management
    fn config_path(&self) -> Option<std::path::PathBuf> { None }
    fn is_injected(&self) -> bool { false }
    fn get_injection_payload(&self) -> Option<serde_json::Value> { None }
    fn inject(&self) -> Result<()> { Err(anyhow::anyhow!("Not implemented for this agent")) }
    fn uninstall(&self) -> Result<()> { Err(anyhow::anyhow!("Not implemented for this agent")) }
}
