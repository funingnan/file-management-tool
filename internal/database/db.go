package database

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	_ "modernc.org/sqlite"
)

// DB 封装 SQLite 数据库操作
type DB struct {
	conn *sql.DB
}

// New 初始化数据库连接并执行迁移
func New(dataDir string) (*DB, error) {
	if err := os.MkdirAll(dataDir, 0755); err != nil {
		return nil, fmt.Errorf("创建数据目录失败: %w", err)
	}

	dbPath := filepath.Join(dataDir, "data.db")
	conn, err := sql.Open("sqlite", dbPath+"?_journal_mode=WAL&_busy_timeout=5000")
	if err != nil {
		return nil, fmt.Errorf("打开数据库失败: %w", err)
	}

	db := &DB{conn: conn}
	if err := db.migrate(); err != nil {
		conn.Close()
		return nil, fmt.Errorf("数据库迁移失败: %w", err)
	}

	return db, nil
}

// Close 关闭数据库连接
func (db *DB) Close() error {
	return db.conn.Close()
}

// migrate 创建/更新表结构
func (db *DB) migrate() error {
	// 先建基础表
	schema := `
	CREATE TABLE IF NOT EXISTS documents (
		id         INTEGER PRIMARY KEY AUTOINCREMENT,
		path       TEXT UNIQUE NOT NULL,
		filename   TEXT NOT NULL,
		title      TEXT DEFAULT '',
		file_type  TEXT DEFAULT 'pdf',
		file_size  INTEGER DEFAULT 0,
		mod_time   DATETIME DEFAULT CURRENT_TIMESTAMP,
		content    TEXT DEFAULT '',
		created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
		indexed_at DATETIME DEFAULT CURRENT_TIMESTAMP
	);

	CREATE TABLE IF NOT EXISTS tags (
		id   INTEGER PRIMARY KEY AUTOINCREMENT,
		name TEXT UNIQUE NOT NULL
	);

	CREATE TABLE IF NOT EXISTS document_tags (
		document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
		tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
		source      TEXT DEFAULT 'manual',
		PRIMARY KEY (document_id, tag_id)
	);

	CREATE TABLE IF NOT EXISTS tag_relations (
		tag_a_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
		tag_b_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
		weight   INTEGER DEFAULT 1,
		PRIMARY KEY (tag_a_id, tag_b_id)
	);

	CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path);
	CREATE INDEX IF NOT EXISTS idx_documents_size ON documents(file_size);
	CREATE INDEX IF NOT EXISTS idx_document_tags_doc ON document_tags(document_id);
	CREATE INDEX IF NOT EXISTS idx_document_tags_tag ON document_tags(tag_id);
	`
	_, err := db.conn.Exec(schema)
	if err != nil {
		return err
	}

	// 兼容旧数据库：添加新列（已存在则忽略）
	db.conn.Exec(`ALTER TABLE documents ADD COLUMN file_type TEXT DEFAULT 'pdf'`)
	db.conn.Exec(`ALTER TABLE documents ADD COLUMN file_size INTEGER DEFAULT 0`)
	db.conn.Exec(`ALTER TABLE documents ADD COLUMN mod_time DATETIME DEFAULT CURRENT_TIMESTAMP`)
	return nil
}

// ---------- Document 操作 ----------

// UpsertDocument 插入或更新文档记录（按 path 去重）
func (db *DB) UpsertDocument(path, filename, title, fileType string, fileSize int64, modTime time.Time) (int64, error) {
	query := `
		INSERT INTO documents (path, filename, title, file_type, file_size, mod_time, indexed_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(path) DO UPDATE SET
			filename = excluded.filename,
			title = excluded.title,
			file_type = excluded.file_type,
			file_size = excluded.file_size,
			mod_time = excluded.mod_time,
			indexed_at = excluded.indexed_at
		RETURNING id
	`
	var id int64
	err := db.conn.QueryRow(query, path, filename, title, fileType, fileSize, modTime, time.Now()).Scan(&id)
	return id, err
}

// UpsertDocuments 批量插入/更新文档，在单个事务中执行
func (db *DB) UpsertDocuments(docs []DocumentInput) (int, error) {
	if len(docs) == 0 {
		return 0, nil
	}

	tx, err := db.conn.Begin()
	if err != nil {
		return 0, fmt.Errorf("开始事务失败: %w", err)
	}
	defer tx.Rollback()

	query := `
		INSERT INTO documents (path, filename, title, file_type, file_size, mod_time, indexed_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(path) DO UPDATE SET
			filename = excluded.filename,
			title = excluded.title,
			file_type = excluded.file_type,
			file_size = excluded.file_size,
			mod_time = excluded.mod_time,
			indexed_at = excluded.indexed_at
	`

	stmt, err := tx.Prepare(query)
	if err != nil {
		return 0, fmt.Errorf("准备语句失败: %w", err)
	}
	defer stmt.Close()

	now := time.Now()
	successCount := 0
	for _, doc := range docs {
		_, err := stmt.Exec(doc.Path, doc.Filename, doc.Title, doc.FileType, doc.FileSize, doc.ModTime, now)
		if err != nil {
			continue // 跳过失败的记录
		}
		successCount++
	}

	if err := tx.Commit(); err != nil {
		return 0, fmt.Errorf("提交事务失败: %w", err)
	}

	return successCount, nil
}

// ListDocuments 查询文档列表，可按标签和搜索文本过滤
// ListDocuments 查询文档列表，可按标签、搜索文本、文件类型过滤
// untagged=true 时只返回没有任何标签的文档
// fileTypes 为空时不过滤类型，如 ["pdf","docx","xlsx","pptx"]
// 搜索支持模糊匹配（非连续字符匹配），按匹配度排序
func (db *DB) ListDocuments(tagIDs []int64, searchText string, untagged bool, fileTypes []string) ([]Document, error) {
	query := `SELECT DISTINCT d.id, d.path, d.filename, d.title, d.file_type, d.file_size, d.mod_time, d.created_at, d.indexed_at
		FROM documents d`
	args := []interface{}{}
	joins := ""
	where := " WHERE 1=1"

	if untagged {
		where += ` AND d.id NOT IN (SELECT document_id FROM document_tags)`
	} else if len(tagIDs) > 0 {
		placeholders := ""
		for i, tid := range tagIDs {
			if i > 0 {
				placeholders += ","
			}
			placeholders += "?"
			args = append(args, tid)
		}
		joins += ` INNER JOIN document_tags dt ON d.id = dt.document_id`
		where += fmt.Sprintf(` AND dt.tag_id IN (%s)`, placeholders)
	}

	if len(fileTypes) > 0 {
		placeholders := ""
		for i, ft := range fileTypes {
			if i > 0 {
				placeholders += ","
			}
			placeholders += "?"
			args = append(args, ft)
		}
		where += fmt.Sprintf(` AND d.file_type IN (%s)`, placeholders)
	}

	// 构建搜索条件（宽松匹配：只要有任何匹配就返回）
	if searchText != "" {
		// 使用 OR 连接每个字符，只要有任何字符出现就匹配
		charConditions := []string{}
		charArgs := []interface{}{}
		
		for _, r := range strings.ToLower(searchText) {
			char := string(r)
			charConditions = append(charConditions, "d.filename LIKE ? OR d.path LIKE ?")
			charArgs = append(charArgs, "%"+char+"%", "%"+char+"%")
		}
		
		// 标签匹配
		like := "%" + searchText + "%"
		
		where += fmt.Sprintf(` AND ((%s) OR d.id IN (
			SELECT dt2.document_id FROM document_tags dt2
			INNER JOIN tags t ON dt2.tag_id = t.id
			WHERE t.name LIKE ?
		))`, strings.Join(charConditions, " OR "))
		
		args = append(args, charArgs...)
		args = append(args, like)
	}

	rows, err := db.conn.Query(query+joins+where, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var docs []Document
	for rows.Next() {
		var doc Document
		if err := rows.Scan(&doc.ID, &doc.Path, &doc.Filename, &doc.Title, &doc.FileType, &doc.FileSize, &doc.ModTime, &doc.CreatedAt, &doc.IndexedAt); err != nil {
			return nil, err
		}
		docs = append(docs, doc)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	// 如果有搜索文本，按匹配度排序
	if searchText != "" && len(docs) > 0 {
		sort.Slice(docs, func(i, j int) bool {
			scoreI := calculateMatchScore(docs[i].Filename, docs[i].Path, searchText)
			scoreJ := calculateMatchScore(docs[j].Filename, docs[j].Path, searchText)
			return scoreI > scoreJ // 分数高的排前面
		})
	}

	return docs, nil
}

// GetDocument 获取单个文档详情（含标签）
func (db *DB) GetDocument(id int64) (*DocumentDetail, error) {
	var doc DocumentDetail
	err := db.conn.QueryRow(
		`SELECT id, path, filename, title, file_type, file_size, mod_time, created_at, indexed_at FROM documents WHERE id = ?`, id,
	).Scan(&doc.ID, &doc.Path, &doc.Filename, &doc.Title, &doc.FileType, &doc.FileSize, &doc.ModTime, &doc.CreatedAt, &doc.IndexedAt)
	if err != nil {
		return nil, err
	}

	rows, err := db.conn.Query(
		`SELECT t.id, t.name FROM tags t
		 INNER JOIN document_tags dt ON t.id = dt.tag_id
		 WHERE dt.document_id = ? ORDER BY t.name`, id,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var tag Tag
		if err := rows.Scan(&tag.ID, &tag.Name); err != nil {
			return nil, err
		}
		doc.Tags = append(doc.Tags, tag)
	}
	return &doc, rows.Err()
}

// DeleteDocument 删除文档
func (db *DB) DeleteDocument(id int64) error {
	_, err := db.conn.Exec(`DELETE FROM documents WHERE id = ?`, id)
	return err
}

// ---------- Tag 操作 ----------

// EnsureTag 确保标签存在，返回标签 ID
func (db *DB) EnsureTag(name string) (int64, error) {
	_, err := db.conn.Exec(`INSERT OR IGNORE INTO tags (name) VALUES (?)`, name)
	if err != nil {
		return 0, err
	}
	var id int64
	err = db.conn.QueryRow(`SELECT id FROM tags WHERE name = ?`, name).Scan(&id)
	return id, err
}

// ListTags 列出所有标签及使用次数
func (db *DB) ListTags() ([]TagWithCount, error) {
	rows, err := db.conn.Query(`
		SELECT t.id, t.name, COUNT(dt.document_id) as cnt
		FROM tags t
		LEFT JOIN document_tags dt ON t.id = dt.tag_id
		GROUP BY t.id
		ORDER BY cnt DESC, t.name
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var tags []TagWithCount
	for rows.Next() {
		var t TagWithCount
		if err := rows.Scan(&t.ID, &t.Name, &t.Count); err != nil {
			return nil, err
		}
		tags = append(tags, t)
	}
	return tags, rows.Err()
}

// AddTagToDocument 给文档添加标签
func (db *DB) AddTagToDocument(docID, tagID int64, source string) error {
	_, err := db.conn.Exec(
		`INSERT OR IGNORE INTO document_tags (document_id, tag_id, source) VALUES (?, ?, ?)`,
		docID, tagID, source,
	)
	return err
}

// RemoveTagFromDocument 从文档移除标签
func (db *DB) RemoveTagFromDocument(docID, tagID int64) error {
	_, err := db.conn.Exec(
		`DELETE FROM document_tags WHERE document_id = ? AND tag_id = ?`,
		docID, tagID,
	)
	return err
}

// DeleteTag 全局删除标签
func (db *DB) DeleteTag(tagID int64) error {
	_, err := db.conn.Exec(`DELETE FROM tags WHERE id = ?`, tagID)
	return err
}

// RenameTag 重命名标签
func (db *DB) RenameTag(tagID int64, newName string) error {
	_, err := db.conn.Exec(`UPDATE tags SET name = ? WHERE id = ?`, newName, tagID)
	return err
}

// ---------- Graph 操作 ----------

// GetDocumentGraph 生成 PDF 关联网络数据
func (db *DB) GetDocumentGraph() (*GraphData, error) {
	// 获取有标签的文档
	rows, err := db.conn.Query(`
		SELECT DISTINCT d.id, d.filename, COUNT(dt.tag_id) as tag_count
		FROM documents d
		INNER JOIN document_tags dt ON d.id = dt.document_id
		GROUP BY d.id
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var nodes []GraphNode
	nodeSet := make(map[int64]bool)
	for rows.Next() {
		var n GraphNode
		if err := rows.Scan(&n.ID, &n.Label, &n.Size); err != nil {
			return nil, err
		}
		if n.Size < 1 {
			n.Size = 1
		}
		nodes = append(nodes, n)
		nodeSet[n.ID] = true
	}

	// 查找共享标签的文档对
	edgeRows, err := db.conn.Query(`
		SELECT dt1.document_id, dt2.document_id, COUNT(*) as shared
		FROM document_tags dt1
		INNER JOIN document_tags dt2 ON dt1.tag_id = dt2.tag_id AND dt1.document_id < dt2.document_id
		GROUP BY dt1.document_id, dt2.document_id
	`)
	if err != nil {
		return nil, err
	}
	defer edgeRows.Close()

	var edges []GraphEdge
	for edgeRows.Next() {
		var e GraphEdge
		if err := edgeRows.Scan(&e.From, &e.To, &e.Weight); err != nil {
			return nil, err
		}
		if nodeSet[e.From] && nodeSet[e.To] {
			edges = append(edges, e)
		}
	}

	return &GraphData{Nodes: nodes, Edges: edges}, nil
}

// GetTagGraph 生成标签关联网络数据
func (db *DB) GetTagGraph() (*GraphData, error) {
	// 获取标签及使用次数
	rows, err := db.conn.Query(`
		SELECT t.id, t.name, COUNT(dt.document_id) as cnt
		FROM tags t
		INNER JOIN document_tags dt ON t.id = dt.tag_id
		GROUP BY t.id
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var nodes []GraphNode
	nodeSet := make(map[int64]bool)
	for rows.Next() {
		var n GraphNode
		if err := rows.Scan(&n.ID, &n.Label, &n.Size); err != nil {
			return nil, err
		}
		if n.Size < 1 {
			n.Size = 1
		}
		nodes = append(nodes, n)
		nodeSet[n.ID] = true
	}

	// 查找共现在同一文档的标签对
	edgeRows, err := db.conn.Query(`
		SELECT dt1.tag_id, dt2.tag_id, COUNT(*) as cnt
		FROM document_tags dt1
		INNER JOIN document_tags dt2 ON dt1.document_id = dt2.document_id AND dt1.tag_id < dt2.tag_id
		GROUP BY dt1.tag_id, dt2.tag_id
	`)
	if err != nil {
		return nil, err
	}
	defer edgeRows.Close()

	var edges []GraphEdge
	for edgeRows.Next() {
		var e GraphEdge
		if err := edgeRows.Scan(&e.From, &e.To, &e.Weight); err != nil {
			return nil, err
		}
		if nodeSet[e.From] && nodeSet[e.To] {
			edges = append(edges, e)
		}
	}

	return &GraphData{Nodes: nodes, Edges: edges}, nil
}

// CountDocuments 统计文档总数
func (db *DB) CountDocuments() (int, error) {
	var count int
	err := db.conn.QueryRow(`SELECT COUNT(*) FROM documents`).Scan(&count)
	return count, err
}

// CountUntaggedDocuments 统计无标签文档数（可按文件类型过滤）
func (db *DB) CountUntaggedDocuments(fileTypes []string) (int, error) {
	query := `SELECT COUNT(*) FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags)`
	args := []interface{}{}
	if len(fileTypes) > 0 {
		placeholders := ""
		for i, ft := range fileTypes {
			if i > 0 {
				placeholders += ","
			}
			placeholders += "?"
			args = append(args, ft)
		}
		query += fmt.Sprintf(` AND file_type IN (%s)`, placeholders)
	}
	var count int
	err := db.conn.QueryRow(query, args...).Scan(&count)
	return count, err
}

// ---------- 智能匹配（文件改名/移动后保留标签） ----------

// LostDocument 丢失的文档记录（文件路径已不存在）
type LostDocument struct {
	ID       int64
	Path     string
	Filename string
	FileSize int64
	ModTime  time.Time
}

// ListLostDocuments 返回数据库中路径已不存在的文档
func (db *DB) ListLostDocuments() ([]LostDocument, error) {
	rows, err := db.conn.Query(`SELECT id, path, filename, file_size, mod_time FROM documents`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var lost []LostDocument
	for rows.Next() {
		var ld LostDocument
		if err := rows.Scan(&ld.ID, &ld.Path, &ld.Filename, &ld.FileSize, &ld.ModTime); err != nil {
			continue
		}
		if _, err := os.Stat(ld.Path); os.IsNotExist(err) {
			lost = append(lost, ld)
		}
	}
	return lost, rows.Err()
}

// UpdateDocumentPath 更新文档路径（智能匹配成功后调用）
func (db *DB) UpdateDocumentPath(docID int64, newPath, newFilename string, fileSize int64, modTime time.Time) error {
	_, err := db.conn.Exec(
		`UPDATE documents SET path = ?, filename = ?, file_size = ?, mod_time = ?, indexed_at = ? WHERE id = ?`,
		newPath, newFilename, fileSize, modTime, time.Now(), docID,
	)
	return err
}

// RemoveDocument 从数据库移除文档记录（不删真实文件）
func (db *DB) RemoveDocument(docID int64) error {
	_, err := db.conn.Exec(`DELETE FROM documents WHERE id = ?`, docID)
	return err
}

// GetDocumentByPath 通过文件路径获取文档
func (db *DB) GetDocumentByPath(path string) (*Document, error) {
	var doc Document
	err := db.conn.QueryRow(
		`SELECT id, path, filename, title, file_type, file_size, mod_time, created_at, indexed_at FROM documents WHERE path = ?`, path,
	).Scan(&doc.ID, &doc.Path, &doc.Filename, &doc.Title, &doc.FileType, &doc.FileSize, &doc.ModTime, &doc.CreatedAt, &doc.IndexedAt)
	if err != nil {
		return nil, err
	}
	return &doc, nil
}

// UpsertDocumentFromPath 从文件路径创建或更新文档记录（右键打标签时自动入库）
func (db *DB) UpsertDocumentFromPath(path string) (*Document, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, fmt.Errorf("无法读取文件信息: %w", err)
	}
	filename := filepath.Base(path)
	fileType := inferFileType(path)

	id, err := db.UpsertDocument(path, filename, "", fileType, info.Size(), info.ModTime())
	if err != nil {
		return nil, err
	}
	return &Document{
		ID:       id,
		Path:     path,
		Filename: filename,
		FileType: fileType,
		FileSize: info.Size(),
		ModTime:  info.ModTime(),
	}, nil
}

// CountByFileType 按文件类型统计数量
func (db *DB) CountByFileType() (map[string]int, error) {
	rows, err := db.conn.Query(`SELECT file_type, COUNT(*) FROM documents GROUP BY file_type`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	counts := make(map[string]int)
	for rows.Next() {
		var ft string
		var cnt int
		if err := rows.Scan(&ft, &cnt); err != nil {
			continue
		}
		counts[ft] = cnt
	}
	return counts, rows.Err()
}

// inferFileType 从文件路径推断文件类型
func inferFileType(path string) string {
	lower := strings.ToLower(path)
	switch {
	case strings.HasSuffix(lower, ".pdf"):
		return "pdf"
	case strings.HasSuffix(lower, ".doc") || strings.HasSuffix(lower, ".docx"):
		return "docx"
	case strings.HasSuffix(lower, ".xls") || strings.HasSuffix(lower, ".xlsx"):
		return "xlsx"
	case strings.HasSuffix(lower, ".ppt") || strings.HasSuffix(lower, ".pptx"):
		return "pptx"
	case strings.HasSuffix(lower, ".jpg") || strings.HasSuffix(lower, ".jpeg") ||
		strings.HasSuffix(lower, ".png") || strings.HasSuffix(lower, ".gif") ||
		strings.HasSuffix(lower, ".bmp") || strings.HasSuffix(lower, ".webp") ||
		strings.HasSuffix(lower, ".svg") || strings.HasSuffix(lower, ".ico") ||
		strings.HasSuffix(lower, ".tiff") || strings.HasSuffix(lower, ".tif"):
		return "image"
	case strings.HasSuffix(lower, ".mp4") || strings.HasSuffix(lower, ".avi") ||
		strings.HasSuffix(lower, ".mkv") || strings.HasSuffix(lower, ".mov") ||
		strings.HasSuffix(lower, ".wmv") || strings.HasSuffix(lower, ".flv") ||
		strings.HasSuffix(lower, ".webm") || strings.HasSuffix(lower, ".m4v"):
		return "video"
	default:
		return "other"
	}
}

// buildFuzzyPattern 构建模糊匹配模式
// 例如 "abc" 变成 "%a%b%c%"
func buildFuzzyPattern(search string) string {
	var builder strings.Builder
	builder.WriteString("%")
	for _, r := range strings.ToLower(search) {
		builder.WriteRune(r)
		builder.WriteString("%")
	}
	return builder.String()
}

// buildPartialMatchPattern 构建部分匹配模式
// 每个字符都要出现在文本中（不要求连续或按顺序）
func buildPartialMatchConditions(search string) (string, []interface{}) {
	if len(search) == 0 {
		return "", nil
	}
	
	conditions := []string{}
	args := []interface{}{}
	
	for _, r := range strings.ToLower(search) {
		char := string(r)
		conditions = append(conditions, "(filename LIKE ? OR path LIKE ?)")
		args = append(args, "%"+char+"%", "%"+char+"%")
	}
	
	return "(" + strings.Join(conditions, " AND ") + ")", args
}

// calculateMatchScore 计算搜索文本与文件名/路径的匹配分数
// 分数越高表示匹配度越好
// 支持部分匹配：只要有任何字符匹配就给分
func calculateMatchScore(filename, path, searchText string) int {
	score := 0
	searchLower := strings.ToLower(searchText)
	filenameLower := strings.ToLower(filename)
	pathLower := strings.ToLower(path)

	// 1. 完全匹配文件名 (最高分)
	if filenameLower == searchLower {
		return 1000
	}

	// 2. 文件名包含完整搜索文本
	if strings.Contains(filenameLower, searchLower) {
		score += 500
		// 越靠前分数越高
		idx := strings.Index(filenameLower, searchLower)
		score += 100 - idx
		if score < 0 {
			score = 500
		}
	}

	// 3. 路径包含完整搜索文本
	if strings.Contains(pathLower, searchLower) {
		score += 200
	}

	// 4. 模糊匹配：按顺序匹配的字符数
	fuzzyFilename := calcFuzzyScore(filenameLower, searchLower)
	fuzzyPath := calcFuzzyScore(pathLower, searchLower)
	
	if fuzzyFilename > 0 {
		score += fuzzyFilename
	}
	if fuzzyPath > 0 {
		score += fuzzyPath / 2
	}

	// 5. 部分匹配：统计匹配的字符数（不要求顺序）
	matchCount := countMatchedChars(filenameLower, searchLower)
	if matchCount > 0 {
		// 匹配字符数占搜索长度的比例 * 100
		score += (matchCount * 100) / len(searchLower)
	}

	// 6. 文件名长度越短，相对匹配度越高
	if score > 0 {
		lengthBonus := 100 - len(filenameLower)
		if lengthBonus < 0 {
			lengthBonus = 0
		}
		score += lengthBonus / 10
	}

	return score
}

// countMatchedChars 统计 search 中有多少字符出现在 text 中（不要求顺序）
func countMatchedChars(text, search string) int {
	textRunes := []rune(text)
	searchRunes := []rune(search)
	
	matched := 0
	for _, sr := range searchRunes {
		for _, tr := range textRunes {
			if sr == tr {
				matched++
				break
			}
		}
	}
	return matched
}

// calcFuzzyScore 计算模糊匹配分数
// 检查 search 中的每个字符是否按顺序出现在 text 中
// 支持部分匹配：返回匹配字符数的分数
func calcFuzzyScore(text, search string) int {
	if len(search) == 0 {
		return 0
	}

	searchRunes := []rune(search)
	textRunes := []rune(text)
	
	searchIdx := 0
	matchCount := 0
	lastMatchIdx := -1
	consecutiveBonus := 0

	for i, r := range textRunes {
		if searchIdx < len(searchRunes) && r == searchRunes[searchIdx] {
			matchCount++
			// 连续匹配加分
			if lastMatchIdx == i-1 {
				consecutiveBonus += 10
			}
			lastMatchIdx = i
			searchIdx++
		}
	}

	// 返回部分匹配分数：匹配字符数 * 10 + 连续加分
	if matchCount > 0 {
		return matchCount*10 + consecutiveBonus
	}

	return 0
}
