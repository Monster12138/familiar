use async_trait::async_trait;

#[async_trait]
pub trait AgentHook: std::fmt::Debug + Send + Sync {
    async fn execute(&self) -> anyhow::Result<()>;
}
