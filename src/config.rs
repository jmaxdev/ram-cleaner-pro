use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub auto_purge_enabled: bool,
    pub threshold_percent: f32,
    pub interval_minutes: u64,
    pub cooldown_seconds: u64,
    pub purge_working_sets: bool,
    pub purge_standby_list: bool,
    pub purge_modified_list: bool,
    pub purge_system_cache: bool,
    pub start_minimized: bool,
    pub notify_on_purge: bool,
    #[serde(default = "default_check_updates")]
    pub check_updates_enabled: bool,
    #[serde(default)]
    pub skipped_version: Option<String>,
}

fn default_check_updates() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_purge_enabled: true,
            threshold_percent: 20.0,
            interval_minutes: 30,
            cooldown_seconds: 60,
            purge_working_sets: true,
            purge_standby_list: true,
            purge_modified_list: true,
            purge_system_cache: true,
            start_minimized: false,
            notify_on_purge: true,
            check_updates_enabled: true,
            skipped_version: None,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let dir = PathBuf::from(appdata).join(".ramcleaner");
            let _ = fs::create_dir_all(&dir);
            dir.join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut config) = toml::from_str::<AppConfig>(&content) {
                    config.threshold_percent = config.threshold_percent.max(20.0);
                    config.interval_minutes = config.interval_minutes.max(7);
                    config.cooldown_seconds = config.cooldown_seconds.max(30);
                    return config;
                }
            }
        }
        let config = Self::default();
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}
