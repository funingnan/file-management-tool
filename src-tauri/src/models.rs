use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentDetail {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub title: String,
    pub file_type: String,
    pub file_size: i64,
    pub mod_time: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagWithCount {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNode {
    pub id: i64,
    pub label: String,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    pub from: i64,
    pub to: i64,
    pub weight: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanFolderResult {
    pub total: usize,
    pub new: usize,
    pub updated: usize,
    pub lost: usize,
    pub relocated: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_enabled_types")]
    pub enabled_types: Vec<String>,
}

fn default_enabled_types() -> Vec<String> {
    vec![
        "pdf".to_string(),
        "docx".to_string(),
        "xlsx".to_string(),
        "pptx".to_string(),
        "image".to_string(),
        "video".to_string(),
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled_types: default_enabled_types(),
        }
    }
}
