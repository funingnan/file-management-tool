use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanResult {
    pub path: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i64,
    pub mod_time: String,
}

fn get_all_extensions() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert("pdf", vec!["pdf"]);
    m.insert("docx", vec!["docx", "doc"]);
    m.insert("xlsx", vec!["xlsx", "xls"]);
    m.insert("pptx", vec!["pptx", "ppt"]);
    m.insert("image", vec!["jpg", "jpeg", "png", "gif", "bmp", "webp", "svg"]);
    m.insert("video", vec!["mp4", "avi", "mkv", "mov", "wmv", "flv"]);
    m
}

fn build_ext_map(enabled_types: &[String]) -> HashMap<String, String> {
    let all = get_all_extensions();
    let mut ext_map = HashMap::new();
    for (type_id, exts) in &all {
        if enabled_types.contains(&type_id.to_string()) {
            for ext in exts {
                ext_map.insert(ext.to_string(), type_id.to_string());
            }
        }
    }
    ext_map
}

pub fn get_file_type(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let all = get_all_extensions();
    for (type_id, exts) in &all {
        if exts.contains(&ext.as_str()) {
            return type_id.to_string();
        }
    }
    "unknown".to_string()
}

pub fn scan_folder(folder_path: &str, enabled_types: &[String]) -> Vec<ScanResult> {
    let ext_map = build_ext_map(enabled_types);
    let abs_path = std::fs::canonicalize(folder_path).unwrap_or_else(|_| folder_path.into());
    let mut results = Vec::new();

    for entry in WalkDir::new(&abs_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let ext = entry.path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if let Some(file_type) = ext_map.get(&ext) {
            let metadata = entry.metadata().ok();
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let mod_time = metadata
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            results.push(ScanResult {
                path: entry.path().to_string_lossy().to_string(),
                filename: entry.file_name().to_string_lossy().to_string(),
                file_type: file_type.clone(),
                file_size,
                mod_time,
            });
        }
    }
    results
}
