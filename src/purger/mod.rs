pub mod win32;

use crate::config::AppConfig;
pub use win32::{get_memory_stats, is_admin, MemoryStats};

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PurgeResult {
    pub initial_used_bytes: u64,
    pub final_used_bytes: u64,
    pub bytes_freed: u64,
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
    let bytes_freed = initial_stats.used_bytes.saturating_sub(final_stats.used_bytes);
    let mb_freed = initial_stats.used_mb.saturating_sub(final_stats.used_mb);

    PurgeResult {
        initial_used_bytes: initial_stats.used_bytes,
        final_used_bytes: final_stats.used_bytes,
        bytes_freed,
        initial_used_mb: initial_stats.used_mb,
        final_used_mb: final_stats.used_mb,
        mb_freed,
        processes_trimmed,
        levels_executed,
        errors,
    }
}
