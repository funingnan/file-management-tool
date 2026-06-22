use crate::models::Settings;
use std::path::PathBuf;

pub fn settings_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join("settings.json")
}

pub fn load_settings(data_dir: &str) -> Settings {
    let path = settings_path(data_dir);
    if let Ok(content) = std::fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Settings::default()
    }
}

pub fn save_settings(data_dir: &str, settings: &Settings) -> Result<(), String> {
    let path = settings_path(data_dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
