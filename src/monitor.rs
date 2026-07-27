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
    pub used_bytes: u64,
    pub used_mb: u64,
}

pub struct MonitorState {
    pub config: AppConfig,
    pub history: VecDeque<RamDataPoint>,
    pub last_stats: MemoryStats,
    pub last_purge_time: Option<Instant>,
    pub last_purge_result: Option<PurgeResult>,
    pub total_freed_bytes_session: u64,
    pub total_freed_mb_session: u64,
    pub last_update_check: Option<Instant>,
    pub pending_update: Option<crate::updater::UpdateInfo>,
    pub is_checking_update: bool,
    pub update_error: Option<String>,
    pub update_rx: Option<std::sync::mpsc::Receiver<Result<Option<crate::updater::UpdateInfo>, String>>>,
}

impl MonitorState {
    pub fn new(config: AppConfig) -> Self {
        let stats = get_memory_stats();
        let mut history = VecDeque::with_capacity(60);
        history.push_back(RamDataPoint {
            timestamp_secs: 0,
            usage_percent: stats.usage_percent,
            used_bytes: stats.used_bytes,
            used_mb: stats.used_mb,
        });

        Self {
            config,
            history,
            last_stats: stats,
            last_purge_time: None,
            last_purge_result: None,
            total_freed_bytes_session: 0,
            total_freed_mb_session: 0,
            last_update_check: None,
            pending_update: None,
            is_checking_update: false,
            update_error: None,
            update_rx: None,
        }
    }

    pub fn check_update_async(&mut self) {
        let skipped = self.config.skipped_version.clone();
        self.last_update_check = Some(Instant::now());
        self.is_checking_update = true;
        self.update_error = None;

        let (tx, rx) = std::sync::mpsc::channel();
        self.update_rx = Some(rx);

        std::thread::spawn(move || {
            let res = crate::updater::check_for_update(skipped.as_deref());
            let _ = tx.send(res);
        });
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
            used_bytes: stats.used_bytes,
            used_mb: stats.used_mb,
        });

        if let Some(ref rx) = self.update_rx {
            if let Ok(res) = rx.try_recv() {
                self.is_checking_update = false;
                match res {
                    Ok(Some(info)) => {
                        self.pending_update = Some(info);
                    }
                    Ok(None) => {
                        self.pending_update = None;
                    }
                    Err(e) => {
                        self.update_error = Some(e);
                    }
                }
                self.update_rx = None;
            }
        }

        if self.config.check_updates_enabled {
            let should_check = self
                .last_update_check
                .map(|t| t.elapsed() >= Duration::from_secs(3 * 3600))
                .unwrap_or(true);

            if should_check && !self.is_checking_update && self.update_rx.is_none() {
                self.check_update_async();
            }
        }

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
        self.total_freed_bytes_session += res.bytes_freed;
        self.total_freed_mb_session += res.mb_freed;
        self.last_purge_result = Some(res.clone());
        self.last_stats = get_memory_stats();
        res
    }
}

pub type SharedMonitorState = Arc<RwLock<MonitorState>>;
