mod app;
mod database;
mod graph;
mod models;
mod scanner;
mod settings;

use std::path::PathBuf;

fn main() {
    let exec_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exec_dir = exec_path.parent().unwrap_or(PathBuf::from(".").as_path());
    let data_dir = exec_dir.join("data").to_string_lossy().to_string();
    let db = match database::Database::new(&data_dir) { Ok(db) => db, Err(e) => { eprintln!("数据库初始化失败: {}", e); std::process::exit(1); } };
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 800.0]).with_min_inner_size([1180.0, 600.0]).with_title("PDF 知识库"),
        ..Default::default()
    };
    let data_dir_clone = data_dir.clone();
    let _ = eframe::run_native("PDF Knowledge Base", native_options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, db, data_dir_clone)))));
}
