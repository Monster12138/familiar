use crate::event::AgentEvent;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
    history: Arc<RwLock<VecDeque<AgentEvent>>>,
    max_history: usize,
}

impl EventBus {
    pub fn new(capacity: usize, max_history: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
        }
    }

    pub async fn publish(&self, event: AgentEvent) -> Result<()> {
        // Record in history
        let mut history = self.history.write().await;
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(event.clone());
        
        // Broadcast to subscribers (ignore error if no receivers are active yet)
        let _ = self.sender.send(event);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }

    pub async fn get_history(&self) -> Vec<AgentEvent> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }
}
