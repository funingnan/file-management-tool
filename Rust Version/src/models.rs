use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64, pub path: String, pub filename: String, pub file_type: String,
    pub file_size: i64, pub mod_time: NaiveDateTime, pub created_at: NaiveDateTime, pub indexed_at: NaiveDateTime,
}
#[derive(Debug, Clone)]
pub struct Tag { pub id: i64, pub name: String }
#[derive(Debug, Clone)]
pub struct TagWithCount { pub id: i64, pub name: String, pub count: i64 }
#[derive(Debug, Clone)]
pub struct DocumentDetail { pub doc: Document, pub tags: Vec<Tag> }
#[derive(Debug, Clone)]
pub struct ScanResult { pub path: String, pub filename: String, pub file_type: String, pub file_size: i64, pub mod_time: NaiveDateTime }
#[derive(Debug, Clone)]
pub struct GraphNode { pub id: i64, pub label: String, pub size: f32, pub x: f32, pub y: f32, pub vx: f32, pub vy: f32 }
#[derive(Debug, Clone)]
pub struct GraphEdge { pub from: i64, pub to: i64, pub weight: f32 }
#[derive(Debug, Clone)]
pub struct GraphData { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> }
impl Default for GraphData { fn default() -> Self { Self { nodes: Vec::new(), edges: Vec::new() } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings { #[serde(default = "default_enabled_types")] pub enabled_types: Vec<String> }
fn default_enabled_types() -> Vec<String> { vec!["pdf".into(), "docx".into(), "xlsx".into(), "pptx".into(), "image".into(), "video".into()] }
impl Default for Settings { fn default() -> Self { Self { enabled_types: default_enabled_types() } } }

pub struct FileTypeConfig { pub ext: &'static str, pub type_name: &'static str, pub label: &'static str, pub color: [u8; 3] }
pub const ALL_FILE_TYPES: &[FileTypeConfig] = &[
    FileTypeConfig { ext: ".pdf", type_name: "pdf", label: "PDF", color: [209, 52, 56] },
    FileTypeConfig { ext: ".doc", type_name: "docx", label: "Word", color: [43, 87, 154] },
    FileTypeConfig { ext: ".docx", type_name: "docx", label: "Word", color: [43, 87, 154] },
    FileTypeConfig { ext: ".xls", type_name: "xlsx", label: "Excel", color: [33, 115, 70] },
    FileTypeConfig { ext: ".xlsx", type_name: "xlsx", label: "Excel", color: [33, 115, 70] },
    FileTypeConfig { ext: ".ppt", type_name: "pptx", label: "PPT", color: [196, 62, 28] },
    FileTypeConfig { ext: ".pptx", type_name: "pptx", label: "PPT", color: [196, 62, 28] },
    FileTypeConfig { ext: ".jpg", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".jpeg", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".png", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".gif", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".bmp", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".webp", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".svg", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".ico", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".tiff", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".tif", type_name: "image", label: "IMG", color: [107, 47, 160] },
    FileTypeConfig { ext: ".mp4", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".avi", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".mkv", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".mov", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".wmv", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".flv", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".webm", type_name: "video", label: "VID", color: [45, 45, 45] },
    FileTypeConfig { ext: ".m4v", type_name: "video", label: "VID", color: [45, 45, 45] },
];

pub fn get_file_type_color(type_name: &str) -> [u8; 3] { for ft in ALL_FILE_TYPES { if ft.type_name == type_name { return ft.color; } } [136, 136, 136] }
pub fn get_file_type_label(type_name: &str) -> &str { for ft in ALL_FILE_TYPES { if ft.type_name == type_name { return ft.label; } } "?" }
pub fn get_supported_types() -> Vec<String> { let mut seen = std::collections::HashSet::new(); let mut t = Vec::new(); for ft in ALL_FILE_TYPES { if seen.insert(ft.type_name) { t.push(ft.type_name.to_string()); } } t }
