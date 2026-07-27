use semver::Version;
use serde_json::Value;
use std::fs;
use std::time::Duration;

pub const GITHUB_RELEASES_LATEST_URL: &str =
    "https://api.github.com/repos/jmaxdev/ram-cleaner-pro/releases/latest";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
}

pub fn check_for_update(skipped_version: Option<&str>) -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("RAMPurgerPro")
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(GITHUB_RELEASES_LATEST_URL)
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP Error {}", response.status()));
    }

    let release: Value = response.json().map_err(|e| e.to_string())?;

    let tag_name = release["tag_name"]
        .as_str()
        .ok_or_else(|| "Missing tag_name in release".to_string())?;

    let latest_version_str = tag_name.trim_start_matches('v');

    if let Some(skipped) = skipped_version {
        if skipped == latest_version_str {
            return Ok(None);
        }
    }

    let current_version_str = env!("CARGO_PKG_VERSION");

    let latest_ver = Version::parse(latest_version_str).map_err(|e| e.to_string())?;
    let current_ver = Version::parse(current_version_str).map_err(|e| e.to_string())?;

    if latest_ver > current_ver {
        let release_notes = release["body"].as_str().unwrap_or("").to_string();

        let mut download_url = String::new();
        if let Some(assets) = release["assets"].as_array() {
            for asset in assets {
                if let Some(name) = asset["name"].as_str() {
                    if name.contains("ram") || name.ends_with(".exe") {
                        if let Some(url) = asset["browser_download_url"].as_str() {
                            download_url = url.to_string();
                            break;
                        }
                    }
                }
            }
            if download_url.is_empty() && !assets.is_empty() {
                if let Some(url) = assets[0]["browser_download_url"].as_str() {
                    download_url = url.to_string();
                }
            }
        }

        if download_url.is_empty() {
            download_url = format!(
                "https://github.com/jmaxdev/ram-cleaner-pro/releases/download/v{}/ram-pro.exe",
                latest_version_str
            );
        }

        Ok(Some(UpdateInfo {
            version: latest_version_str.to_string(),
            release_notes,
            download_url,
        }))
    } else {
        Ok(None)
    }
}

pub fn download_and_apply_update(download_url: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("RAMPurgerPro")
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(download_url)
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }

    let bytes = response.bytes().map_err(|e| e.to_string())?;

    let temp_dir = std::env::temp_dir();
    let temp_exe = temp_dir.join("ram_pro_new.exe");

    fs::write(&temp_exe, &bytes).map_err(|e| e.to_string())?;

    self_replace::self_replace(&temp_exe).map_err(|e| e.to_string())?;

    let _ = fs::remove_file(temp_exe);

    Ok(())
}
