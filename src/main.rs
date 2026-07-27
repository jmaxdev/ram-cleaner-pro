#![windows_subsystem = "windows"]

mod cli;
mod config;
mod monitor;
mod purger;
mod ui;
mod updater;

use clap::Parser;
use cli::CliArgs;
use config::AppConfig;
use monitor::MonitorState;
use parking_lot::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use ui::RamPurgerApp;

fn main() {
    let args = CliArgs::parse();
    let mut config = AppConfig::load_or_default();

    if let Some(th) = args.threshold {
        config.threshold_percent = th;
    }
    if let Some(inv) = args.interval {
        config.interval_minutes = inv;
    }

    if args.check_update {
        println!("Checking for updates on GitHub...");
        match updater::check_for_update(None) {
            Ok(Some(info)) => {
                println!("New version available: v{}", info.version);
                println!("Release Notes:\n{}", info.release_notes);
            }
            Ok(None) => {
                println!("RAM Purger Pro is up to date (v{}).", env!("CARGO_PKG_VERSION"));
            }
            Err(e) => {
                println!("Error checking updates: {}", e);
            }
        }
        return;
    }

    if args.purge_now {
        println!("==================================================");
        println!("RAM MEMORY PURGE IN PROGRESS (WINDOWS NT API)");
        println!("==================================================");
        let res = purger::execute_purge(&config);
        println!("Initial RAM used : {}", purger::format_bytes(res.initial_used_bytes));
        println!("Final RAM used   : {}", purger::format_bytes(res.final_used_bytes));
        println!("RAM FREED        : {}", purger::format_bytes(res.bytes_freed));
        println!("Processes trimmed: {}", res.processes_trimmed);
        println!("Levels executed  : {}", res.levels_executed.join(", "));
        if !res.errors.is_empty() {
            println!("Warnings: {}", res.errors.join("; "));
        }
        return;
    }

    if args.status {
        let stats = purger::get_memory_stats();
        println!("{{");
        println!("  \"total_bytes\": {},", stats.total_bytes);
        println!("  \"used_bytes\": {},", stats.used_bytes);
        println!("  \"free_bytes\": {},", stats.free_bytes);
        println!("  \"total_formatted\": \"{}\",", purger::format_bytes(stats.total_bytes));
        println!("  \"used_formatted\": \"{}\",", purger::format_bytes(stats.used_bytes));
        println!("  \"free_formatted\": \"{}\",", purger::format_bytes(stats.free_bytes));
        println!("  \"total_mb\": {},", stats.total_mb);
        println!("  \"used_mb\": {},", stats.used_mb);
        println!("  \"free_mb\": {},", stats.free_mb);
        println!("  \"usage_percent\": {:.2},", stats.usage_percent);
        println!("  \"auto_purge_enabled\": {},", config.auto_purge_enabled);
        println!("  \"threshold_percent\": {:.1},", config.threshold_percent);
        println!("  \"interval_minutes\": {}", config.interval_minutes);
        println!("}}");
        return;
    }

    if args.daemon {
        println!("Starting RAM Purger background daemon service...");
        let monitor = Arc::new(RwLock::new(MonitorState::new(config)));

        loop {
            std::thread::sleep(Duration::from_secs(2));
            let mut mon = monitor.write();
            if let Some(res) = mon.update() {
                println!(
                    "[{}] Auto-purge completed: Freed {} of RAM (Processes: {})",
                    chrono_timestamp(),
                    purger::format_bytes(res.bytes_freed),
                    res.processes_trimmed
                );
            }
        }
    }

    let monitor_state = Arc::new(RwLock::new(MonitorState::new(config)));
    let is_visible = Arc::new(AtomicBool::new(true));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RAM Purger")
            .with_inner_size([380.0, 620.0])
            .with_min_inner_size([380.0, 620.0])
            .with_max_inner_size([380.0, 620.0])
            .with_resizable(false)
            .with_maximize_button(false),
        ..Default::default()
    };

    let mon_app = Arc::clone(&monitor_state);
    let vis_app = Arc::clone(&is_visible);

    if let Err(e) = eframe::run_native(
        "RAM Purger",
        options,
        Box::new(|cc| Ok(Box::new(RamPurgerApp::new(cc, mon_app, vis_app)))),
    ) {
        eprintln!("Failed to launch GUI interface: {}", e);
    }
}

fn chrono_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}
