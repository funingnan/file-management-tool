use crate::models::*;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(data_dir: &str) -> SqlResult<Self> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            rusqlite::Error::InvalidParameterName(format!("创建数据目录失败: {}", e))
        })?;
        let db_path = PathBuf::from(data_dir).join("data.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS documents (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                path       TEXT UNIQUE NOT NULL,
                filename   TEXT NOT NULL,
                title      TEXT DEFAULT '',
                file_type  TEXT DEFAULT 'pdf',
                file_size  INTEGER DEFAULT 0,
                mod_time   TEXT DEFAULT '',
                content    TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                indexed_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL
            );
            CREATE TABLE IF NOT EXISTS document_tags (
                document_id INTEGER NOT NULL,
                tag_id      INTEGER NOT NULL,
                source      TEXT DEFAULT 'manual',
                PRIMARY KEY (document_id, tag_id),
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path);
            CREATE INDEX IF NOT EXISTS idx_document_tags_doc ON document_tags(document_id);
            CREATE INDEX IF NOT EXISTS idx_document_tags_tag ON document_tags(tag_id);
            "
        )?;
        Ok(())
    }

    pub fn upsert_document(&self, path: &str, filename: &str, file_type: &str, file_size: i64, mod_time: &str) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO documents (path, filename, file_type, file_size, mod_time)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET
                file_size = excluded.file_size,
                mod_time = excluded.mod_time,
                indexed_at = datetime('now')",
            params![path, filename, file_type, file_size, mod_time],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_documents(&self, tag_ids: &[i64], search_text: &str, untagged: bool, file_types: &[String]) -> SqlResult<Vec<Document>> {
        let mut sql = String::from(
            "SELECT DISTINCT d.id, d.path, d.filename, d.file_type, d.file_size
             FROM documents d"
        );
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !tag_ids.is_empty() {
            sql.push_str(" INNER JOIN document_tags dt ON d.id = dt.document_id");
            let placeholders: Vec<String> = tag_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            conditions.push(format!("dt.tag_id IN ({})", placeholders.join(",")));
            for id in tag_ids {
                param_values.push(Box::new(*id));
            }
        }

        if untagged {
            conditions.push("d.id NOT IN (SELECT document_id FROM document_tags)".to_string());
        }

        if !file_types.is_empty() {
            let idx = param_values.len() + 1;
            let placeholders: Vec<String> = file_types.iter().enumerate().map(|(i, _)| format!("?{}", idx + i)).collect();
            conditions.push(format!("d.file_type IN ({})", placeholders.join(",")));
            for ft in file_types {
                param_values.push(Box::new(ft.clone()));
            }
        }

        if !search_text.is_empty() {
            let idx = param_values.len() + 1;
            conditions.push(format!("(d.filename LIKE ?{0} OR d.path LIKE ?{0} OR d.id IN (SELECT dt2.document_id FROM document_tags dt2 INNER JOIN tags t2 ON dt2.tag_id = t2.id WHERE t2.name LIKE ?{0}))", idx));
            param_values.push(Box::new(format!("%{}%", search_text)));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY d.indexed_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get(2)?,
                file_type: row.get(3)?,
                file_size: row.get(4)?,
            })
        })?;

        let mut docs = Vec::new();
        for row in rows {
            docs.push(row?);
        }
        Ok(docs)
    }

    pub fn get_document(&self, id: i64) -> SqlResult<Option<DocumentDetail>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, filename, title, file_type, file_size, mod_time FROM documents WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DocumentDetail {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get(2)?,
                title: row.get(3)?,
                file_type: row.get(4)?,
                file_size: row.get(5)?,
                mod_time: row.get(6)?,
                tags: Vec::new(),
            })
        })?;

        if let Some(mut doc) = rows.next().transpose()? {
            doc.tags = self.get_document_tags(id)?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    fn get_document_tags(&self, doc_id: i64) -> SqlResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name FROM tags t
             INNER JOIN document_tags dt ON t.id = dt.tag_id
             WHERE dt.document_id = ?1"
        )?;
        let rows = stmt.query_map(params![doc_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_document(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM document_tags WHERE document_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn ensure_tag(&self, name: &str) -> SqlResult<i64> {
        self.conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![name])?;
        let mut stmt = self.conn.prepare("SELECT id FROM tags WHERE name = ?1")?;
        let id: i64 = stmt.query_row(params![name], |row| row.get(0))?;
        Ok(id)
    }

    pub fn list_tags(&self) -> SqlResult<Vec<TagWithCount>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, COUNT(dt.document_id) as cnt
             FROM tags t LEFT JOIN document_tags dt ON t.id = dt.tag_id
             GROUP BY t.id ORDER BY cnt DESC, t.name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TagWithCount {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn add_tag_to_document(&self, doc_id: i64, tag_id: i64) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO document_tags (document_id, tag_id, source) VALUES (?1, ?2, 'manual')",
            params![doc_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_document(&self, doc_id: i64, tag_id: i64) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM document_tags WHERE document_id = ?1 AND tag_id = ?2",
            params![doc_id, tag_id],
        )?;
        Ok(())
    }

    pub fn delete_tag(&self, tag_id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM document_tags WHERE tag_id = ?1", params![tag_id])?;
        self.conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])?;
        Ok(())
    }

    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> SqlResult<()> {
        self.conn.execute("UPDATE tags SET name = ?1 WHERE id = ?2", params![new_name, tag_id])?;
        Ok(())
    }

    pub fn count_documents(&self) -> SqlResult<i64> {
        self.conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
    }

    pub fn count_untagged_documents(&self, file_types: &[String]) -> SqlResult<i64> {
        let sql = if file_types.is_empty() {
            "SELECT COUNT(*) FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags)".to_string()
        } else {
            let placeholders: Vec<String> = file_types.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            format!(
                "SELECT COUNT(*) FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags) AND file_type IN ({})",
                placeholders.join(",")
            )
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = file_types.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))
    }

    pub fn count_by_file_type(&self) -> SqlResult<std::collections::HashMap<String, i64>> {
        let mut stmt = self.conn.prepare("SELECT file_type, COUNT(*) FROM documents GROUP BY file_type")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn get_document_by_path(&self, path: &str) -> SqlResult<Option<Document>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, filename, file_type, file_size FROM documents WHERE path = ?1"
        )?;
        let mut rows = stmt.query_map(params![path], |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get(2)?,
                file_type: row.get(3)?,
                file_size: row.get(4)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn list_lost_documents(&self) -> SqlResult<Vec<Document>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, filename, file_type, file_size FROM documents"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                filename: row.get(2)?,
                file_type: row.get(3)?,
                file_size: row.get(4)?,
            })
        })?;
        let mut lost = Vec::new();
        for row in rows {
            let doc = row?;
            if !std::path::Path::new(&doc.path).exists() {
                lost.push(doc);
            }
        }
        Ok(lost)
    }

    pub fn update_document_path(&self, doc_id: i64, new_path: &str, new_filename: &str, file_size: i64, mod_time: &str) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE documents SET path = ?1, filename = ?2, file_size = ?3, mod_time = ?4, indexed_at = datetime('now') WHERE id = ?5",
            params![new_path, new_filename, file_size, mod_time, doc_id],
        )?;
        Ok(())
    }

    pub fn get_document_graph(&self) -> SqlResult<GraphData> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.filename, COUNT(dt.tag_id) as tag_count
             FROM documents d INNER JOIN document_tags dt ON d.id = dt.document_id
             GROUP BY d.id"
        )?;
        let mut nodes = Vec::new();
        let mut node_set = std::collections::HashSet::new();
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let label: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            Ok((id, label, size.max(1)))
        })?;
        for row in rows {
            let (id, label, size) = row?;
            nodes.push(GraphNode { id, label, size });
            node_set.insert(id);
        }

        let mut edge_stmt = self.conn.prepare(
            "SELECT dt1.document_id, dt2.document_id, COUNT(*) as shared
             FROM document_tags dt1
             INNER JOIN document_tags dt2 ON dt1.tag_id = dt2.tag_id AND dt1.document_id < dt2.document_id
             GROUP BY dt1.document_id, dt2.document_id"
        )?;
        let mut edges = Vec::new();
        let edge_rows = edge_stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in edge_rows {
            let (from, to, weight) = row?;
            if node_set.contains(&from) && node_set.contains(&to) {
                edges.push(GraphEdge { from, to, weight });
            }
        }

        Ok(GraphData { nodes, edges })
    }

    pub fn get_tag_graph(&self) -> SqlResult<GraphData> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, COUNT(dt.document_id) as cnt
             FROM tags t INNER JOIN document_tags dt ON t.id = dt.tag_id
             GROUP BY t.id"
        )?;
        let mut nodes = Vec::new();
        let mut node_set = std::collections::HashSet::new();
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let label: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            Ok((id, label, size.max(1)))
        })?;
        for row in rows {
            let (id, label, size) = row?;
            nodes.push(GraphNode { id, label, size });
            node_set.insert(id);
        }

        let mut edge_stmt = self.conn.prepare(
            "SELECT dt1.tag_id, dt2.tag_id, COUNT(*) as cnt
             FROM document_tags dt1
             INNER JOIN document_tags dt2 ON dt1.document_id = dt2.document_id AND dt1.tag_id < dt2.tag_id
             GROUP BY dt1.tag_id, dt2.tag_id"
        )?;
        let mut edges = Vec::new();
        let edge_rows = edge_stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in edge_rows {
            let (from, to, weight) = row?;
            if node_set.contains(&from) && node_set.contains(&to) {
                edges.push(GraphEdge { from, to, weight });
            }
        }

        Ok(GraphData { nodes, edges })
    }

    pub fn get_document_tags_by_path(&self, path: &str) -> SqlResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name FROM tags t
             INNER JOIN document_tags dt ON t.id = dt.tag_id
             INNER JOIN documents d ON dt.document_id = d.id
             WHERE d.path = ?1"
        )?;
        let rows = stmt.query_map(params![path], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_or_create_document_by_path(&self, path: &str) -> SqlResult<i64> {
        if let Some(doc) = self.get_document_by_path(path)? {
            return Ok(doc.id);
        }
        let filename = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_type = crate::scanner::get_file_type(path);
        let (file_size, mod_time) = std::fs::metadata(path)
            .map(|m| (m.len() as i64, m.modified().ok().map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            }).unwrap_or_default()))
            .unwrap_or((0, String::new()));
        self.upsert_document(path, &filename, &file_type, file_size, &mod_time)
    }

    pub fn wal_checkpoint(&self) -> SqlResult<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }
}
