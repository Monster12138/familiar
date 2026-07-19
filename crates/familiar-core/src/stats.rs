use crate::state::DailyStats;
use crate::event::AgentEvent;
use std::sync::{Arc, RwLock};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct StatisticsEngine {
    stats: Arc<RwLock<DailyStats>>,
}

impl Default for StatisticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StatisticsEngine {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(DailyStats {
                interactions: 0,
                active_time_seconds: 0,
                tasks_completed: 0,
            })),
        }
    }

    pub fn process_event(&self, _event: &AgentEvent) -> Result<()> {
        let mut stats = self.stats.write().unwrap();
        stats.interactions += 1;
        Ok(())
    }

    pub fn get_daily_stats(&self) -> DailyStats {
        self.stats.read().unwrap().clone()
    }
}
