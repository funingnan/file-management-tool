use std::path::PathBuf;
use crate::models::Settings;
fn settings_path(data_dir: &str) -> PathBuf { PathBuf::from(data_dir).join("settings.json") }
pub fn load_settings(data_dir: &str) -> Settings { let p = settings_path(data_dir); match std::fs::read_to_string(&p) { Ok(c) => serde_json::from_str(&c).unwrap_or_default(), Err(_) => Settings::default() } }
pub fn save_settings(data_dir: &str, s: &Settings) -> Result<(), String> { let p = settings_path(data_dir); if let Some(par) = p.parent() { std::fs::create_dir_all(par).map_err(|e| e.to_string())?; } let j = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?; std::fs::write(&p, j).map_err(|e| e.to_string())?; Ok(()) }
