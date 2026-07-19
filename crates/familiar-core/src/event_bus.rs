use tokio::sync::broadcast;
use std::sync::{Arc, RwLock};
use std::collections::VecDeque;
use anyhow::Result;
use crate::event::AgentEvent;

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
    history: Arc<RwLock<VecDeque<AgentEvent>>>,
    history_capacity: usize,
}

impl EventBus {
    pub fn new(capacity: usize, history_capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(history_capacity))),
            history_capacity,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: AgentEvent) -> Result<()> {
        let mut history = self.history.write().unwrap();
        if history.len() >= self.history_capacity {
            history.pop_front();
        }
        history.push_back(event.clone());
        
        // Ignore send errors if there are no subscribers
        let _ = self.sender.send(event);
        Ok(())
    }

    pub fn get_history(&self) -> Vec<AgentEvent> {
        self.history.read().unwrap().iter().cloned().collect()
    }
}
