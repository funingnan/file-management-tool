use std::collections::{HashMap, HashSet};
use eframe::egui;
use crate::database::Database;
use crate::graph::layout_graph;
use crate::models::*;
use crate::scanner;
use crate::settings;

pub struct App {
    db: Database, data_dir: String, settings: Settings,
    documents: Vec<Document>, all_tags: Vec<TagWithCount>,
    selected_doc_id: Option<i64>, selected_doc_detail: Option<DocumentDetail>,
    selected_doc_ids: HashSet<i64>, search_text: String,
    filter_mode: FilterMode, file_type_filter: String, active_tag_ids: Vec<i64>,
    view_mode: ViewMode, graph_mode: GraphMode, graph_data: Option<GraphData>,
    tag_input: String, batch_tag_input: String, show_settings: bool,
    toast: Option<(String, f64)>, type_counts: HashMap<String, i64>, untagged_count: i64, total_count: i64,
}
#[derive(PartialEq)] enum FilterMode { All, Folder, Untagged }
#[derive(PartialEq)] enum ViewMode { List, Graph }
#[derive(PartialEq)] enum GraphMode { Document, Tag }

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, db: Database, data_dir: String) -> Self {
        let settings = settings::load_settings(&data_dir);
        let mut app = Self { db, data_dir, settings, documents: Vec::new(), all_tags: Vec::new(), selected_doc_id: None, selected_doc_detail: None, selected_doc_ids: HashSet::new(), search_text: String::new(), filter_mode: FilterMode::All, file_type_filter: "all".into(), active_tag_ids: Vec::new(), view_mode: ViewMode::List, graph_mode: GraphMode::Document, graph_data: None, tag_input: String::new(), batch_tag_input: String::new(), show_settings: false, toast: None, type_counts: HashMap::new(), untagged_count: 0, total_count: 0 };
        app.refresh_all(); app
    }
    fn refresh_all(&mut self) { self.refresh_tags(); self.refresh_documents(); self.refresh_counts(); }
    fn refresh_tags(&mut self) { self.all_tags = self.db.list_tags().unwrap_or_default(); }
    fn refresh_documents(&mut self) {
        let ft = if self.file_type_filter == "all" { self.settings.enabled_types.clone() } else { vec![self.file_type_filter.clone()] };
        self.documents = self.db.list_documents(&self.active_tag_ids, &self.search_text, self.filter_mode == FilterMode::Untagged, &ft).unwrap_or_default();
    }
    fn refresh_counts(&mut self) { self.total_count = self.db.count_documents().unwrap_or(0); self.untagged_count = self.db.count_untagged(&self.settings.enabled_types).unwrap_or(0); self.type_counts = self.db.count_by_file_type().unwrap_or_default().into_iter().collect(); }
    fn show_toast(&mut self, msg: &str) { self.toast = Some((msg.to_string(), 0.0)); }
    fn select_document(&mut self, id: i64) { self.selected_doc_id = Some(id); self.selected_doc_detail = self.db.get_document(id).ok(); }
    fn smart_relocate(&self, scanned: &[ScanResult]) -> i64 {
        let db_count = self.db.count_documents().unwrap_or(0);
        if scanned.len() as i64 >= db_count { return 0; }
        let lost = match self.db.list_lost_documents() { Ok(l) => l, Err(_) => return 0 };
        if lost.is_empty() { return 0; }
        let fp: HashMap<(i64,i64), &ScanResult> = scanned.iter().map(|s| ((s.file_size, s.mod_time.timestamp()), s)).collect();
        let mut r = 0;
        for d in &lost { let k = (d.file_size, d.mod_time.timestamp()); if let Some(m) = fp.get(&k) { if self.db.update_document_path(d.id, m.path, m.filename, m.file_size, m.mod_time).is_ok() { r += 1; } } } r
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((_, ref mut t)) = self.toast { *t += ctx.input(|i| i.unstable_dt); if *t > 2.0 { self.toast = None; } }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                if ui.button("📂 扫描文件").clicked() {
                    if let Some(f) = rfd::FileDialog::new().pick_folder() {
                        let p = f.display().to_string();
                        match scanner::scan_folder(&p, &self.settings.enabled_types) { r if !r.is_empty() => { let _ = self.db.upsert_documents(&r); let rc = self.smart_relocate(&r); self.refresh_all(); self.show_toast(if rc > 0 { &format!("找回 {} 个文件", rc) } else { "已完成加载" }); }, _ => { self.show_toast("未发现支持的文件"); } }
                    }
                }
                if ui.button("📁 选择路径").clicked() {
                    if let Some(f) = rfd::FileDialog::new().pick_folder() {
                        let p = f.display().to_string();
                        let r = scanner::scan_folder(&p, &self.settings.enabled_types);
                        if !r.is_empty() { let _ = self.db.upsert_documents(&r); let _ = self.smart_relocate(&r); }
                        self.filter_mode = FilterMode::Folder; self.file_type_filter = "all".into(); self.active_tag_ids.clear(); self.refresh_all(); self.show_toast("已完成加载");
                    }
                }
                ui.add_space(20.0);
                let sr = ui.add_sized([175.0, 24.0], egui::TextEdit::singleline(&mut self.search_text).hint_text("搜索文件名或标签..."));
                if sr.changed() { self.refresh_documents(); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(format!("{} 个文件", self.total_count));
                    if ui.selectable_label(self.view_mode == ViewMode::List, "📋").clicked() { self.view_mode = ViewMode::List; }
                    if ui.selectable_label(self.view_mode == ViewMode::Graph, "🕸").clicked() { self.view_mode = ViewMode::Graph; self.graph_data = None; }
                });
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label("v0.2.0");
                if let Some((msg, _)) = &self.toast { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.add_space(8.0); ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(92,184,92))); }); }
            });
        });

        egui::SidePanel::left("left").resizable(false).exact_width(220.0).show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading(egui::RichText::new("  文件类型").size(14.0).strong());
            ui.add_space(4.0);
            if ui.selectable_label(self.filter_mode == FilterMode::All && self.file_type_filter == "all", format!("  ALL  所有文件 ({})", self.total_count)).clicked() { self.filter_mode = FilterMode::All; self.file_type_filter = "all".into(); self.active_tag_ids.clear(); self.refresh_documents(); }
            for t in &["pdf","docx","xlsx","pptx","image","video"] {
                if self.settings.enabled_types.contains(&t.to_string()) {
                    let c = self.type_counts.get(*t).copied().unwrap_or(0);
                    if ui.selectable_label(self.file_type_filter == *t, format!("  {}  {} ({})", get_file_type_label(t), t, c)).clicked() { self.file_type_filter = t.to_string(); self.filter_mode = FilterMode::All; self.refresh_documents(); }
                }
            }
            if ui.selectable_label(self.filter_mode == FilterMode::Untagged, format!("  ?  未分类 ({})", self.untagged_count)).clicked() { self.filter_mode = FilterMode::Untagged; self.file_type_filter = "all".into(); self.active_tag_ids.clear(); self.refresh_documents(); }
            ui.separator(); ui.add_space(4.0);
            ui.horizontal(|ui| { ui.heading(egui::RichText::new("  标签").size(14.0).strong()); if !self.active_tag_ids.is_empty() { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.small_button("✕ 清除").clicked() { self.active_tag_ids.clear(); self.refresh_documents(); } }); } });
            egui::ScrollArea::vertical().show(ui, |ui| {
                for tag in &self.all_tags {
                    let sel = self.active_tag_ids.contains(&tag.id);
                    if ui.selectable_label(sel, format!("  # {} ({})", tag.name, tag.count)).clicked() { if sel { self.active_tag_ids.retain(|&id| id != tag.id); } else { self.active_tag_ids.push(tag.id); } self.refresh_documents(); }
                }
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| { ui.add_space(4.0); if ui.button("⚙ 设置").clicked() { self.show_settings = true; } });
        });

        if self.show_settings {
            egui::Window::new("设置").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0,0.0]).show(ctx, |ui| {
                ui.heading("文件类型"); ui.add_space(8.0);
                let types = get_supported_types(); let mut changed = false;
                for t in &types { let mut en = self.settings.enabled_types.contains(t); if ui.checkbox(&mut en, format!("{} ({})", get_file_type_label(t), t)).changed() { if en && !self.settings.enabled_types.contains(t) { self.settings.enabled_types.push(t.clone()); } else if !en { self.settings.enabled_types.retain(|x| x != t); } changed = true; } }
                if changed { let _ = settings::save_settings(&self.data_dir, &self.settings); self.refresh_counts(); }
                ui.add_space(8.0); if ui.button("关闭").clicked() { self.show_settings = false; }
            });
        }

        egui::SidePanel::right("right").resizable(false).exact_width(300.0).show(ctx, |ui| {
            ui.add_space(12.0);
            match &self.selected_doc_detail {
                None => { ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("👈 选择一个文件查看详情").color(egui::Color32::GRAY)); }); }
                Some(detail) => {
                    let color = get_file_type_color(&detail.doc.file_type);
                    let label = get_file_type_label(&detail.doc.file_type);
                    ui.horizontal(|ui| { ui.label(egui::RichText::new(format!("[{}]", label)).color(egui::Color32::from_rgb(color[0],color[1],color[2])).strong().size(16.0)); ui.add_space(4.0); ui.label(egui::RichText::new(&detail.doc.filename).strong().size(15.0)); });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&detail.doc.path).size(11.0).color(egui::Color32::GRAY));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| { let path = detail.doc.path.clone(); if ui.button("📄 打开文件").clicked() { let _ = open::that(&path); } if ui.button("📁 打开目录").clicked() { if let Some(p) = std::path::Path::new(&path).parent() { let _ = open::that(p); } } if ui.button("🗑 移除文件").clicked() { let id = detail.doc.id; let _ = self.db.remove_document(id); self.selected_doc_id = None; self.selected_doc_detail = None; self.refresh_all(); } });
                    ui.add_space(12.0); ui.label(egui::RichText::new("标签").strong()); ui.add_space(4.0);
                    let tags = detail.tags.clone(); let mut rm: Option<i64> = None;
                    ui.horizontal_wrapped(|ui| { for t in &tags { if ui.selectable_label(false, format!("# {} ×", t.name)).clicked() { rm = Some(t.id); } } });
                    if let Some(tid) = rm { let did = self.selected_doc_detail.as_ref().unwrap().doc.id; let _ = self.db.remove_tag_from_document(did, tid); self.select_document(did); self.refresh_tags(); }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| { let r = ui.add_sized([200.0,22.0], egui::TextEdit::singleline(&mut self.tag_input).hint_text("输入新标签...")); if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { let name = self.tag_input.trim().to_string(); if !name.is_empty() { let did = self.selected_doc_detail.as_ref().unwrap().doc.id; if let Ok(tid) = self.db.ensure_tag(&name) { let _ = self.db.add_tag_to_document(did, tid); self.tag_input.clear(); self.select_document(did); self.refresh_tags(); } } } });
                    let ci: HashSet<i64> = tags.iter().map(|t| t.id).collect();
                    ui.add_space(8.0); ui.label(egui::RichText::new("可选标签").size(12.0).color(egui::Color32::GRAY));
                    ui.horizontal_wrapped(|ui| { for t in &self.all_tags { if !ci.contains(&t.id) { if ui.small(format!("# {}", t.name)).clicked() { let did = self.selected_doc_detail.as_ref().unwrap().doc.id; let _ = self.db.add_tag_to_document(did, t.id); self.select_document(did); self.refresh_tags(); } } } });
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut sa = self.selected_doc_ids.len() == self.documents.len() && !self.documents.is_empty();
                if ui.checkbox(&mut sa, "全选").changed() { if sa { self.selected_doc_ids = self.documents.iter().map(|d| d.id).collect(); } else { self.selected_doc_ids.clear(); } }
                if !self.selected_doc_ids.is_empty() {
                    ui.label(format!("{} 个已选", self.selected_doc_ids.len()));
                    if ui.button("取消选择").clicked() { self.selected_doc_ids.clear(); }
                    if ui.button("移除文件").clicked() { let ids: Vec<i64> = self.selected_doc_ids.iter().copied().collect(); for id in &ids { let _ = self.db.remove_document(*id); } self.selected_doc_ids.clear(); self.selected_doc_detail = None; self.selected_doc_id = None; self.refresh_all(); }
                    ui.add_sized([120.0,20.0], egui::TextEdit::singleline(&mut self.batch_tag_input).hint_text("标签..."));
                    if ui.button("添加标签").clicked() { let name = self.batch_tag_input.trim().to_string(); if !name.is_empty() { if let Ok(tid) = self.db.ensure_tag(&name) { let ids: Vec<i64> = self.selected_doc_ids.iter().copied().collect(); for id in &ids { let _ = self.db.add_tag_to_document(*id, tid); } self.batch_tag_input.clear(); self.refresh_tags(); self.refresh_documents(); self.show_toast(&format!("已添加标签「{}」", name)); } } }
                }
            });
            ui.separator();
            match self.view_mode {
                ViewMode::List => {
                    let docs = self.documents.clone();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for doc in &docs {
                            let is_sel = self.selected_doc_id == Some(doc.id);
                            let is_chk = self.selected_doc_ids.contains(&doc.id);
                            ui.horizontal(|ui| {
                                ui.add_space(4.0);
                                let mut chk = is_chk;
                                if ui.checkbox(&mut chk, "").changed() { if chk { self.selected_doc_ids.insert(doc.id); } else { self.selected_doc_ids.remove(&doc.id); } }
                                let color = get_file_type_color(&doc.file_type);
                                ui.label(egui::RichText::new(format!("[{}]", get_file_type_label(&doc.file_type))).color(egui::Color32::from_rgb(color[0],color[1],color[2])).strong().size(12.0));
                                ui.add_space(4.0);
                                if ui.selectable_label(is_sel, &doc.filename).clicked() { self.select_document(doc.id); }
                            });
                            ui.separator();
                        }
                    });
                }
                ViewMode::Graph => {
                    ui.horizontal(|ui| { if ui.selectable_label(self.graph_mode == GraphMode::Document, "文档关联").clicked() { self.graph_mode = GraphMode::Document; self.graph_data = None; } if ui.selectable_label(self.graph_mode == GraphMode::Tag, "标签关联").clicked() { self.graph_mode = GraphMode::Tag; self.graph_data = None; } });
                    ui.add_space(4.0);
                    if self.graph_data.is_none() {
                        self.graph_data = Some(match self.graph_mode { GraphMode::Document => self.db.get_document_graph().unwrap_or_default(), GraphMode::Tag => self.db.get_tag_graph().unwrap_or_default() });
                        if let Some(ref mut gd) = self.graph_data { let sz = ui.available_size(); layout_graph(gd, sz.x, sz.y); }
                    }
                    if let Some(ref graph) = self.graph_data {
                        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
                        let rect = resp.rect;
                        for e in &graph.edges { if let (Some(f), Some(t)) = (graph.nodes.iter().find(|n|n.id==e.from), graph.nodes.iter().find(|n|n.id==e.to)) { let fp = egui::pos2(rect.left()+f.x, rect.top()+f.y); let tp = egui::pos2(rect.left()+t.x, rect.top()+t.y); painter.line_segment([fp,tp], egui::Stroke::new((e.weight.sqrt()*0.5).max(0.5).min(3.0), egui::Color32::from_gray(200))); } }
                        for n in &graph.nodes {
                            let pos = egui::pos2(rect.left()+n.x, rect.top()+n.y);
                            let r = (n.size.sqrt()*3.0+4.0).min(20.0);
                            let color = egui::Color32::from_rgb(((n.id*37)%200+55) as u8, ((n.id*73)%200+55) as u8, ((n.id*113)%200+55) as u8);
                            painter.circle_filled(pos, r, color);
                            painter.text(pos+egui::vec2(r+3.0,0.0), egui::Align2::LEFT_CENTER, &n.label, egui::FontId::proportional(11.0), egui::Color32::BLACK);
                            if resp.clicked() { if let Some(cp) = resp.interact_pointer_pos() { let d = (cp.x-pos.x).powi(2)+(cp.y-pos.y).powi(2); if d < (r+5.0).powi(2) { match self.graph_mode { GraphMode::Document => { self.select_document(n.id); } GraphMode::Tag => { self.active_tag_ids.clear(); self.active_tag_ids.push(n.id); self.view_mode = ViewMode::List; self.refresh_documents(); } } } } }
                        }
                    }
                }
            }
        });
    }
}
