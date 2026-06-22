use std::path::Path;
use chrono::NaiveDateTime;
use rusqlite::{Connection, params};
use crate::models::*;

pub struct Database { conn: Connection }

impl Database {
    pub fn new(data_dir: &str) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
        let db_path = Path::new(data_dir).join("data.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("打开数据库失败: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;").map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT UNIQUE NOT NULL, filename TEXT NOT NULL,
                title TEXT DEFAULT '', file_type TEXT DEFAULT 'pdf', file_size INTEGER DEFAULT 0,
                mod_time DATETIME DEFAULT CURRENT_TIMESTAMP, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                indexed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL);
            CREATE TABLE IF NOT EXISTS document_tags (
                document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                source TEXT DEFAULT 'manual', PRIMARY KEY (document_id, tag_id)
            );
            CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path);
            CREATE INDEX IF NOT EXISTS idx_documents_size ON documents(file_size);
            CREATE INDEX IF NOT EXISTS idx_document_tags_doc ON document_tags(document_id);
            CREATE INDEX IF NOT EXISTS idx_document_tags_tag ON document_tags(tag_id);"
        ).map_err(|e| format!("迁移失败: {}", e))?;
        let _ = self.conn.execute_batch("ALTER TABLE documents ADD COLUMN file_type TEXT DEFAULT 'pdf'");
        let _ = self.conn.execute_batch("ALTER TABLE documents ADD COLUMN file_size INTEGER DEFAULT 0");
        let _ = self.conn.execute_batch("ALTER TABLE documents ADD COLUMN mod_time DATETIME DEFAULT CURRENT_TIMESTAMP");
        Ok(())
    }

    pub fn upsert_documents(&self, docs: &[ScanResult]) -> Result<i64, String> {
        let tx = self.conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count: i64 = 0;
        { let mut stmt = tx.prepare("INSERT INTO documents (path,filename,file_type,file_size,mod_time,indexed_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(path) DO UPDATE SET filename=excluded.filename,file_type=excluded.file_type,file_size=excluded.file_size,mod_time=excluded.mod_time,indexed_at=excluded.indexed_at").map_err(|e| e.to_string())?;
          let now = chrono::Local::now().naive_local();
          for d in docs { if stmt.execute(params![d.path,d.filename,d.file_type,d.file_size,d.mod_time,now]).is_ok() { count+=1; } }
        }
        tx.commit().map_err(|e| e.to_string())?; Ok(count)
    }

    pub fn list_documents(&self, tag_ids: &[i64], search_text: &str, untagged: bool, file_types: &[String]) -> Result<Vec<Document>, String> {
        let mut sql = String::from("SELECT DISTINCT d.id,d.path,d.filename,d.file_type,d.file_size,d.mod_time,d.created_at,d.indexed_at FROM documents d");
        let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut wheres = vec!["1=1".to_string()];
        if untagged { wheres.push("d.id NOT IN (SELECT document_id FROM document_tags)".into()); }
        else if !tag_ids.is_empty() {
            sql.push_str(" INNER JOIN document_tags dt ON d.id=dt.document_id");
            let p: Vec<String> = tag_ids.iter().enumerate().map(|(i,_)| format!("?{}", args.len()+i+1)).collect();
            wheres.push(format!("dt.tag_id IN ({})", p.join(",")));
            for t in tag_ids { args.push(Box::new(*t)); }
        }
        if !file_types.is_empty() {
            let p: Vec<String> = file_types.iter().enumerate().map(|(i,_)| format!("?{}", args.len()+i+1)).collect();
            wheres.push(format!("d.file_type IN ({})", p.join(",")));
            for f in file_types { args.push(Box::new(f.clone())); }
        }
        if !search_text.is_empty() {
            let mut cc = Vec::new();
            for ch in search_text.to_lowercase().chars() {
                let idx = args.len()+1;
                cc.push(format!("(d.filename LIKE ?{0} OR d.path LIKE ?{1})", idx, idx+1));
                args.push(Box::new(format!("%{}%", ch)));
                args.push(Box::new(format!("%{}%", ch)));
            }
            let ti = args.len()+1;
            wheres.push(format!("(({}) OR d.id IN (SELECT dt2.document_id FROM document_tags dt2 INNER JOIN tags t ON dt2.tag_id=t.id WHERE t.name LIKE ?{}))", cc.join(" OR "), ti));
            args.push(Box::new(format!("%{}%", search_text)));
        }
        sql.push_str(&format!(" WHERE {} ORDER BY d.filename", wheres.join(" AND ")));
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| Ok(Document { id:row.get(0)?, path:row.get(1)?, filename:row.get(2)?, file_type:row.get(3)?, file_size:row.get(4)?, mod_time:row.get(5)?, created_at:row.get(6)?, indexed_at:row.get(7)? })).map_err(|e| e.to_string())?;
        let mut docs: Vec<Document> = rows.filter_map(|r| r.ok()).collect();
        if !search_text.is_empty() { let st=search_text.to_lowercase(); docs.sort_by(|a,b| calc_match_score(&b.filename,&b.path,&st).cmp(&calc_match_score(&a.filename,&a.path,&st))); }
        Ok(docs)
    }

    pub fn get_document(&self, id: i64) -> Result<DocumentDetail, String> {
        let doc = self.conn.query_row("SELECT id,path,filename,file_type,file_size,mod_time,created_at,indexed_at FROM documents WHERE id=?1", params![id], |row| Ok(Document { id:row.get(0)?, path:row.get(1)?, filename:row.get(2)?, file_type:row.get(3)?, file_size:row.get(4)?, mod_time:row.get(5)?, created_at:row.get(6)?, indexed_at:row.get(7)? })).map_err(|e| e.to_string())?;
        let mut stmt = self.conn.prepare("SELECT t.id,t.name FROM tags t INNER JOIN document_tags dt ON t.id=dt.tag_id WHERE dt.document_id=?1 ORDER BY t.name").map_err(|e| e.to_string())?;
        let tags = stmt.query_map(params![id], |row| Ok(Tag { id:row.get(0)?, name:row.get(1)? })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Ok(DocumentDetail { doc, tags })
    }

    pub fn remove_document(&self, id: i64) -> Result<(), String> { self.conn.execute("DELETE FROM documents WHERE id=?1", params![id]).map_err(|e| e.to_string())?; Ok(()) }
    pub fn count_documents(&self) -> Result<i64, String> { self.conn.query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0)).map_err(|e| e.to_string()) }
    pub fn count_by_file_type(&self) -> Result<Vec<(String, i64)>, String> {
        let mut s = self.conn.prepare("SELECT file_type,COUNT(*) FROM documents GROUP BY file_type").map_err(|e| e.to_string())?;
        Ok(s.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?))).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect())
    }
    pub fn count_untagged(&self, ft: &[String]) -> Result<i64, String> {
        let mut sql = String::from("SELECT COUNT(*) FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags)");
        let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !ft.is_empty() { let p: Vec<String> = ft.iter().enumerate().map(|(i,_)| format!("?{}",i+1)).collect(); sql.push_str(&format!(" AND file_type IN ({})", p.join(","))); for f in ft { args.push(Box::new(f.clone())); } }
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        self.conn.query_row(&sql, refs.as_slice(), |r| r.get(0)).map_err(|e| e.to_string())
    }

    pub fn ensure_tag(&self, name: &str) -> Result<i64, String> { self.conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![name]).map_err(|e| e.to_string())?; self.conn.query_row("SELECT id FROM tags WHERE name=?1", params![name], |r| r.get(0)).map_err(|e| e.to_string()) }
    pub fn list_tags(&self) -> Result<Vec<TagWithCount>, String> {
        let mut s = self.conn.prepare("SELECT t.id,t.name,COUNT(dt.document_id) FROM tags t LEFT JOIN document_tags dt ON t.id=dt.tag_id GROUP BY t.id ORDER BY COUNT(dt.document_id) DESC,t.name").map_err(|e| e.to_string())?;
        Ok(s.query_map([], |r| Ok(TagWithCount { id:r.get(0)?, name:r.get(1)?, count:r.get(2)? })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect())
    }
    pub fn add_tag_to_document(&self, doc_id: i64, tag_id: i64) -> Result<(), String> { self.conn.execute("INSERT OR IGNORE INTO document_tags (document_id,tag_id,source) VALUES (?1,?2,'manual')", params![doc_id,tag_id]).map_err(|e| e.to_string())?; Ok(()) }
    pub fn remove_tag_from_document(&self, doc_id: i64, tag_id: i64) -> Result<(), String> { self.conn.execute("DELETE FROM document_tags WHERE document_id=?1 AND tag_id=?2", params![doc_id,tag_id]).map_err(|e| e.to_string())?; Ok(()) }
    pub fn delete_tag(&self, tag_id: i64) -> Result<(), String> { self.conn.execute("DELETE FROM tags WHERE id=?1", params![tag_id]).map_err(|e| e.to_string())?; Ok(()) }
    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> Result<(), String> { self.conn.execute("UPDATE tags SET name=?1 WHERE id=?2", params![new_name,tag_id]).map_err(|e| e.to_string())?; Ok(()) }

    pub fn list_lost_documents(&self) -> Result<Vec<Document>, String> {
        let mut s = self.conn.prepare("SELECT id,path,filename,file_type,file_size,mod_time,created_at,indexed_at FROM documents").map_err(|e| e.to_string())?;
        Ok(s.query_map([], |r| Ok(Document { id:r.get(0)?, path:r.get(1)?, filename:r.get(2)?, file_type:r.get(3)?, file_size:r.get(4)?, mod_time:r.get(5)?, created_at:r.get(6)?, indexed_at:r.get(7)? })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).filter(|d| !Path::new(&d.path).exists()).collect())
    }
    pub fn update_document_path(&self, id: i64, path: &str, filename: &str, size: i64, mt: NaiveDateTime) -> Result<(), String> {
        self.conn.execute("UPDATE documents SET path=?1,filename=?2,file_size=?3,mod_time=?4,indexed_at=?5 WHERE id=?6", params![path,filename,size,mt,chrono::Local::now().naive_local(),id]).map_err(|e| e.to_string())?; Ok(())
    }

    pub fn get_document_graph(&self) -> Result<GraphData, String> {
        let mut nodes = Vec::new(); let mut ns = std::collections::HashSet::new();
        { let mut s = self.conn.prepare("SELECT DISTINCT d.id,d.filename,COUNT(dt.tag_id) FROM documents d INNER JOIN document_tags dt ON d.id=dt.document_id GROUP BY d.id").map_err(|e| e.to_string())?;
          for r in s.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)? as f32))).map_err(|e| e.to_string())?.flatten() { ns.insert(r.0); nodes.push(GraphNode { id:r.0, label:r.1, size:r.2.max(1.0), x:0.0, y:0.0, vx:0.0, vy:0.0 }); } }
        let mut edges = Vec::new();
        { let mut s = self.conn.prepare("SELECT dt1.document_id,dt2.document_id,COUNT(*) FROM document_tags dt1 INNER JOIN document_tags dt2 ON dt1.tag_id=dt2.tag_id AND dt1.document_id<dt2.document_id GROUP BY dt1.document_id,dt2.document_id").map_err(|e| e.to_string())?;
          for r in s.query_map([], |r| Ok(GraphEdge { from:r.get(0)?, to:r.get(1)?, weight:r.get::<_,i64>(2)? as f32 })).map_err(|e| e.to_string())?.flatten() { if ns.contains(&r.from) && ns.contains(&r.to) { edges.push(r); } } }
        Ok(GraphData { nodes, edges })
    }
    pub fn get_tag_graph(&self) -> Result<GraphData, String> {
        let mut nodes = Vec::new(); let mut ns = std::collections::HashSet::new();
        { let mut s = self.conn.prepare("SELECT t.id,t.name,COUNT(dt.document_id) FROM tags t INNER JOIN document_tags dt ON t.id=dt.tag_id GROUP BY t.id").map_err(|e| e.to_string())?;
          for r in s.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)? as f32))).map_err(|e| e.to_string())?.flatten() { ns.insert(r.0); nodes.push(GraphNode { id:r.0, label:r.1, size:r.2.max(1.0), x:0.0, y:0.0, vx:0.0, vy:0.0 }); } }
        let mut edges = Vec::new();
        { let mut s = self.conn.prepare("SELECT dt1.tag_id,dt2.tag_id,COUNT(*) FROM document_tags dt1 INNER JOIN document_tags dt2 ON dt1.document_id=dt2.document_id AND dt1.tag_id<dt2.tag_id GROUP BY dt1.tag_id,dt2.tag_id").map_err(|e| e.to_string())?;
          for r in s.query_map([], |r| Ok(GraphEdge { from:r.get(0)?, to:r.get(1)?, weight:r.get::<_,i64>(2)? as f32 })).map_err(|e| e.to_string())?.flatten() { if ns.contains(&r.from) && ns.contains(&r.to) { edges.push(r); } } }
        Ok(GraphData { nodes, edges })
    }
}

fn calc_match_score(filename: &str, path: &str, search: &str) -> i64 {
    let fl = filename.to_lowercase(); let pl = path.to_lowercase(); let mut s: i64 = 0;
    if fl == search { return 1000; }
    if let Some(i) = fl.find(search) { s += 500; s += (100 - i as i64).max(0); }
    if pl.contains(search) { s += 200; }
    s += calc_fuzzy(&fl, search); s += calc_fuzzy(&pl, search) / 2;
    let mc = count_matched(&fl, search); if mc > 0 && !search.is_empty() { s += (mc * 100) / search.len() as i64; }
    if s > 0 { s += ((100 - fl.len() as i64).max(0)) / 10; } s
}
fn calc_fuzzy(text: &str, search: &str) -> i64 {
    if search.is_empty() { return 0; }
    let tc: Vec<char> = text.chars().collect(); let sc: Vec<char> = search.chars().collect();
    let mut si = 0; let mut mc = 0; let mut li: i64 = -1; let mut cb: i64 = 0;
    for (i, &c) in tc.iter().enumerate() { if si < sc.len() && c == sc[si] { mc += 1; if li == i as i64 - 1 { cb += 10; } li = i as i64; si += 1; } }
    if mc > 0 { mc * 10 + cb } else { 0 }
}
fn count_matched(text: &str, search: &str) -> i64 { let tc: Vec<char> = text.chars().collect(); search.chars().filter(|c| tc.contains(c)).count() as i64 }
