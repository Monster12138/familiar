use crate::config::FamiliarConfig;
use crate::event::AgentEvent;
use anyhow::Result;

pub trait Plugin: Send + Sync {
    /// Returns the unique name of the plugin
    fn name(&self) -> &str;

    /// Called when the plugin is initialized
    fn initialize(&mut self, config: &FamiliarConfig) -> Result<()>;

    /// Called when an event is published to the event bus
    fn on_event(&mut self, event: &AgentEvent) -> Result<()>;

    /// Called when the plugin is being shut down
    fn shutdown(&mut self) -> Result<()>;
}
