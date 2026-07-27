use crate::config::AppConfig;
use crate::purger::{execute_purge, get_memory_stats, MemoryStats};
pub use crate::purger::PurgeResult;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RamDataPoint {
    pub timestamp_secs: u64,
    pub usage_percent: f32,
    pub used_mb: u64,
}

pub struct MonitorState {
    pub config: AppConfig,
    pub history: VecDeque<RamDataPoint>,
    pub last_stats: MemoryStats,
    pub last_purge_time: Option<Instant>,
    pub last_purge_result: Option<PurgeResult>,
    pub total_freed_mb_session: u64,
}

impl MonitorState {
    pub fn new(config: AppConfig) -> Self {
        let stats = get_memory_stats();
        let mut history = VecDeque::with_capacity(60);
        history.push_back(RamDataPoint {
            timestamp_secs: 0,
            usage_percent: stats.usage_percent,
            used_mb: stats.used_mb,
        });

        Self {
            config,
            history,
            last_stats: stats,
            last_purge_time: None,
            last_purge_result: None,
            total_freed_mb_session: 0,
        }
    }

    pub fn update(&mut self) -> Option<PurgeResult> {
        let stats = get_memory_stats();
        self.last_stats = stats;

        if self.history.len() >= 60 {
            self.history.pop_front();
        }
        self.history.push_back(RamDataPoint {
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            usage_percent: stats.usage_percent,
            used_mb: stats.used_mb,
        });

        let mut purge_triggered = false;
        let now = Instant::now();

        if self.config.auto_purge_enabled {
            let cooldown_passed = self
                .last_purge_time
                .map(|t| now.duration_since(t) >= Duration::from_secs(self.config.cooldown_seconds))
                .unwrap_or(true);

            if cooldown_passed {
                if stats.usage_percent >= self.config.threshold_percent {
                    purge_triggered = true;
                }

                if !purge_triggered && self.config.interval_minutes > 0 {
                    if let Some(last_purge) = self.last_purge_time {
                        if now.duration_since(last_purge)
                            >= Duration::from_secs(self.config.interval_minutes * 60)
                        {
                            purge_triggered = true;
                        }
                    }
                }
            }
        }

        if purge_triggered {
            return Some(self.force_purge());
        }

        None
    }

    pub fn force_purge(&mut self) -> PurgeResult {
        let res = execute_purge(&self.config);
        self.last_purge_time = Some(Instant::now());
        self.total_freed_mb_session += res.mb_freed;
        self.last_purge_result = Some(res.clone());
        self.last_stats = get_memory_stats();
        res
    }
}

pub type SharedMonitorState = Arc<RwLock<MonitorState>>;
