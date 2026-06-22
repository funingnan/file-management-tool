use std::path::Path;
use chrono::NaiveDateTime;
use walkdir::WalkDir;
use crate::models::*;

pub fn scan_folder(folder_path: &str, enabled_types: &[String]) -> Vec<ScanResult> {
    let abs_path = if Path::new(folder_path).is_absolute() { folder_path.to_string() }
    else { std::fs::canonicalize(folder_path).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| folder_path.to_string()) };
    let ext_map = build_ext_map(enabled_types);
    let mut results = Vec::new();
    for entry in WalkDir::new(&abs_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() { continue; }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let ext = match name.rfind('.') { Some(i) => &name[i..], None => continue };
        let file_type = match ext_map.get(ext) { Some(t) => t.clone(), None => continue };
        let metadata = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let mod_time = metadata.modified().ok().and_then(|t| { let d = t.duration_since(std::time::UNIX_EPOCH).ok()?; NaiveDateTime::from_timestamp_opt(d.as_secs() as i64, 0) }).unwrap_or_else(|| chrono::Local::now().naive_local());
        results.push(ScanResult { path: entry.path().to_string_lossy().to_string(), filename: entry.file_name().to_string_lossy().to_string(), file_type, file_size: metadata.len() as i64, mod_time });
    }
    results
}
fn build_ext_map(enabled: &[String]) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    for ft in ALL_FILE_TYPES { if enabled.contains(&ft.type_name.to_string()) { m.insert(ft.ext.to_string(), ft.type_name.to_string()); } } m
}
