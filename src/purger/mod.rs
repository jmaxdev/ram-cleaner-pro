pub mod win32;

use crate::config::AppConfig;
pub use win32::{get_memory_stats, is_admin, MemoryStats};

#[derive(Debug, Clone)]
pub struct PurgeResult {
    pub initial_used_mb: u64,
    pub final_used_mb: u64,
    pub mb_freed: u64,
    pub processes_trimmed: usize,
    pub levels_executed: Vec<String>,
    pub errors: Vec<String>,
}

pub fn execute_purge(config: &AppConfig) -> PurgeResult {
    let initial_stats = get_memory_stats();
    let mut processes_trimmed = 0;
    let mut levels_executed = Vec::new();
    let mut errors = Vec::new();

    if config.purge_working_sets {
        let count = win32::purge_working_sets();
        processes_trimmed = count;
        levels_executed.push(format!("Process Working Sets ({})", count));
    }

    if config.purge_standby_list {
        match win32::purge_standby_list() {
            Ok(_) => levels_executed.push("Standby Memory List".into()),
            Err(e) => errors.push(e),
        }
    }

    if config.purge_modified_list {
        match win32::purge_modified_list() {
            Ok(_) => levels_executed.push("Modified Page List".into()),
            Err(e) => errors.push(e),
        }
    }

    if config.purge_system_cache {
        match win32::purge_system_cache() {
            Ok(_) => levels_executed.push("System File Cache".into()),
            Err(e) => errors.push(e),
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(150));

    let final_stats = get_memory_stats();
    let mb_freed = initial_stats.used_mb.saturating_sub(final_stats.used_mb);

    PurgeResult {
        initial_used_mb: initial_stats.used_mb,
        final_used_mb: final_stats.used_mb,
        mb_freed,
        processes_trimmed,
        levels_executed,
        errors,
    }
}
