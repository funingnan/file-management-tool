mod database;
mod models;
mod scanner;
mod settings;

use models::*;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};

struct AppState {
    db: Mutex<database::Database>,
    data_dir: String,
}

// ========== 标签选择器模式 ==========

#[tauri::command]
fn get_app_mode() -> serde_json::Value {
    serde_json::json!({ "mode": "app", "filePath": "" })
}

#[tauri::command]
fn set_tag_picker_mode(_file_path: String) {}

#[tauri::command]
fn close_tag_picker(app: tauri::AppHandle) {
    app.exit(0);
}

// ========== 设置 ==========

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    settings::load_settings(&state.data_dir)
}

#[tauri::command]
fn save_settings(state: tauri::State<'_, AppState>, new_settings: Settings) -> Result<(), String> {
    settings::save_settings(&state.data_dir, &new_settings)
}

// ========== 文件夹选择与扫描 ==========

#[tauri::command]
fn select_folder(app: tauri::AppHandle) -> Result<String, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    match folder {
        Some(path) => Ok(path.to_string()),
        None => Ok(String::new()),
    }
}

#[tauri::command]
fn scan_folder(state: tauri::State<'_, AppState>, folder_path: String, enabled_types: Vec<String>) -> Result<ScanFolderResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folder(&folder_path, &enabled_types);
    let total = scanned.len();
    let mut new = 0;
    let mut updated = 0;

    for item in &scanned {
        match db.get_document_by_path(&item.path) {
            Ok(Some(_)) => {
                db.upsert_document(&item.path, &item.filename, &item.file_type, item.file_size, &item.mod_time).map_err(|e| e.to_string())?;
                updated += 1;
            }
            Ok(None) => {
                db.upsert_document(&item.path, &item.filename, &item.file_type, item.file_size, &item.mod_time).map_err(|e| e.to_string())?;
                new += 1;
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    let lost_docs = db.list_lost_documents().map_err(|e| e.to_string())?;
    let lost = lost_docs.len();

    let mut relocated = 0usize;
    for lost_doc in &lost_docs {
        for scanned_item in &scanned {
            if scanned_item.filename == lost_doc.filename {
                db.update_document_path(lost_doc.id, &scanned_item.path, &scanned_item.filename, scanned_item.file_size, &scanned_item.mod_time).map_err(|e| e.to_string())?;
                relocated += 1;
                break;
            }
        }
    }

    Ok(ScanFolderResult { total, new, updated, lost, relocated })
}

// ========== 文档操作 ==========

#[tauri::command]
fn list_documents(state: tauri::State<'_, AppState>, tag_ids: Vec<i64>, search_text: String, untagged: bool, file_types: Vec<String>) -> Result<Vec<Document>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_documents(&tag_ids, &search_text, untagged, &file_types).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_document(state: tauri::State<'_, AppState>, id: i64) -> Result<Option<DocumentDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_document(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_document_count(state: tauri::State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count_documents().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_untagged_count(state: tauri::State<'_, AppState>, file_types: Vec<String>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count_untagged_documents(&file_types).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_document(state: tauri::State<'_, AppState>, doc_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_document(doc_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_documents(state: tauri::State<'_, AppState>, doc_ids: Vec<i64>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for id in &doc_ids {
        db.delete_document(*id).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
fn get_file_type_counts(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let map = db.count_by_file_type().map_err(|e| e.to_string())?;
    serde_json::to_value(&map).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_supported_types() -> Vec<String> {
    vec!["pdf".into(), "docx".into(), "xlsx".into(), "pptx".into(), "image".into(), "video".into()]
}

// ========== 标签操作 ==========

#[tauri::command]
fn list_tags(state: tauri::State<'_, AppState>) -> Result<Vec<TagWithCount>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_tags().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_tag_to_document(state: tauri::State<'_, AppState>, doc_id: i64, tag_name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tag_id = db.ensure_tag(&tag_name).map_err(|e| e.to_string())?;
    db.add_tag_to_document(doc_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_tag_from_document(state: tauri::State<'_, AppState>, doc_id: i64, tag_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_tag_from_document(doc_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn batch_add_tag(state: tauri::State<'_, AppState>, doc_ids: Vec<i64>, tag_name: String) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tag_id = db.ensure_tag(&tag_name).map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for id in &doc_ids {
        db.add_tag_to_document(*id, tag_id).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
fn batch_remove_tag_from_documents(state: tauri::State<'_, AppState>, doc_ids: Vec<i64>, tag_id: i64) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for id in &doc_ids {
        db.remove_tag_from_document(*id, tag_id).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
fn delete_tag(state: tauri::State<'_, AppState>, tag_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_tag(tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_tag(state: tauri::State<'_, AppState>, tag_id: i64, new_name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rename_tag(tag_id, &new_name).map_err(|e| e.to_string())
}

// ========== 标签选择器专用 ==========

#[tauri::command]
fn get_document_tags_by_path(state: tauri::State<'_, AppState>, file_path: String) -> Result<Vec<Tag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_document_tags_by_path(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_tag_to_file_path(state: tauri::State<'_, AppState>, file_path: String, tag_name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let doc_id = db.get_or_create_document_by_path(&file_path).map_err(|e| e.to_string())?;
    let tag_id = db.ensure_tag(&tag_name).map_err(|e| e.to_string())?;
    db.add_tag_to_document(doc_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_tag_from_file_path(state: tauri::State<'_, AppState>, file_path: String, tag_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(doc) = db.get_document_by_path(&file_path).map_err(|e| e.to_string())? {
        db.remove_tag_from_document(doc.id, tag_id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ========== 文件操作 ==========

#[tauri::command]
fn open_file(state: tauri::State<'_, AppState>, doc_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(doc) = db.get_document(doc_id).map_err(|e| e.to_string())? {
        open::that(&doc.path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn open_file_location(state: tauri::State<'_, AppState>, doc_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(doc) = db.get_document(doc_id).map_err(|e| e.to_string())? {
        let parent = std::path::Path::new(&doc.path).parent().unwrap_or(std::path::Path::new(&doc.path));
        open::that(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ========== 网络图 ==========

#[tauri::command]
fn get_document_graph(state: tauri::State<'_, AppState>) -> Result<GraphData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_document_graph().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tag_graph(state: tauri::State<'_, AppState>) -> Result<GraphData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_tag_graph().map_err(|e| e.to_string())
}

// ========== 数据导入导出 ==========

#[tauri::command]
fn export_database(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.wal_checkpoint().map_err(|e| e.to_string())?;
    drop(db);

    let src_path = std::path::PathBuf::from(&state.data_dir).join("data.db");

    let save_path = app.dialog().file()
        .set_file_name("data.db")
        .blocking_save_file();

    match save_path {
        Some(path) => {
            let path_str = path.to_string();
            std::fs::copy(&src_path, &path_str).map_err(|e| e.to_string())?;
            Ok(path_str)
        }
        None => Ok(String::new()),
    }
}

#[tauri::command]
fn import_database(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let file = app.dialog().file()
        .add_filter("数据库文件", &["db"])
        .blocking_pick_file();

    let open_path = match file {
        Some(p) => p,
        None => return Ok(false),
    };

    let path_str = open_path.to_string();
    let src_data = std::fs::read(&path_str).map_err(|e| format!("读取文件失败: {}", e))?;
    if src_data.len() < 16 || &src_data[..15] != b"SQLite format 3" {
        return Err("不是有效的数据库文件".to_string());
    }

    {
        let _db = state.db.lock().map_err(|e| e.to_string())?;
    }

    let dst_path = std::path::PathBuf::from(&state.data_dir).join("data.db");
    std::fs::write(&dst_path, &src_data).map_err(|e| format!("写入数据库失败: {}", e))?;
    let _ = std::fs::remove_file(format!("{}-wal", dst_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", dst_path.display()));

    let new_db = database::Database::new(&state.data_dir).map_err(|e| format!("重新打开数据库失败: {}", e))?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    *db = new_db;

    Ok(true)
}

// ========== Tauri 入口 ==========

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resource_dir = app.path().resource_dir().unwrap_or_else(|_| std::env::current_dir().unwrap());
            let data_dir = resource_dir.join("data");
            let data_dir_str = data_dir.to_string_lossy().to_string();
            let db = database::Database::new(&data_dir_str).expect("数据库初始化失败");
            app.manage(AppState {
                db: Mutex::new(db),
                data_dir: data_dir_str,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_mode,
            set_tag_picker_mode,
            close_tag_picker,
            get_settings,
            save_settings,
            select_folder,
            scan_folder,
            list_documents,
            get_document,
            get_document_count,
            get_untagged_count,
            remove_document,
            remove_documents,
            get_file_type_counts,
            get_supported_types,
            list_tags,
            add_tag_to_document,
            remove_tag_from_document,
            batch_add_tag,
            batch_remove_tag_from_documents,
            delete_tag,
            rename_tag,
            get_document_tags_by_path,
            add_tag_to_file_path,
            remove_tag_from_file_path,
            open_file,
            open_file_location,
            get_document_graph,
            get_tag_graph,
            export_database,
            import_database,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
