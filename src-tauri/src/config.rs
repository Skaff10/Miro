use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::model::AppSettings;

pub fn settings_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("com.antigravity.agent-monitor");
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(home)
            .join(".config")
            .join("antigravity-agent-monitor");
    }
    PathBuf::from(".")
}

pub fn settings_file_path() -> PathBuf {
    settings_dir().join("settings.json")
}

pub fn load_settings(path: &Path) -> AppSettings {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let mut parsed = serde_json::from_str::<AppSettings>(&raw).unwrap_or_default();
            // Migrate previous default overlay dimensions to compact box dimensions.
            if parsed.window_width == 392 && parsed.window_height == 520 {
                parsed.window_width = 268;
                parsed.window_height = 230;
            }
            sanitize_settings(parsed)
        }
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let clean = sanitize_settings(settings.clone());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {e}"))?;
    }
    let json = serde_json::to_string_pretty(&clean)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, json).map_err(|e| format!("Failed to write temp settings: {e}"))?;
    fs::rename(&tmp_path, path).map_err(|e| format!("Failed to commit settings file: {e}"))?;
    Ok(())
}

pub fn sanitize_settings(mut settings: AppSettings) -> AppSettings {
    settings.poll_interval_ms = settings.poll_interval_ms.clamp(1000, 30000);
    settings.window_width = 240;
    settings.window_height = settings.window_height.clamp(64, 680);
    if settings.hotkey.trim().is_empty() {
        settings.hotkey = "Ctrl+Shift+A".to_string();
    }
    settings
}
