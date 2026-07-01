package database

import "time"

// DocumentInput 表示批量插入时的文档输入
type DocumentInput struct {
	Path     string
	Filename string
	Title    string
	FileType string
	FileSize int64
	ModTime  time.Time
}

// Document 表示一个文件记录
type Document struct {
	ID        int64     `json:"id"`
	Path      string    `json:"path"`
	Filename  string    `json:"filename"`
	Title     string    `json:"title"`
	FileType  string    `json:"file_type"`  // pdf / docx / xlsx / pptx
	FileSize  int64     `json:"file_size"`  // 文件大小（字节）
	ModTime   time.Time `json:"mod_time"`   // 文件修改时间
	Content   string    `json:"-"`          // 预留
	CreatedAt time.Time `json:"created_at"`
	IndexedAt time.Time `json:"indexed_at"`
}

// Tag 表示一个标签
type Tag struct {
	ID    int64  `json:"id"`
	Name  string `json:"name"`
	Color string `json:"color"`
}

// TagWithCount 带使用次数的标签
type TagWithCount struct {
	ID    int64  `json:"id"`
	Name  string `json:"name"`
	Color string `json:"color"`
	Count int    `json:"count"`
}

// DocumentDetail 文档详情（含标签列表）
type DocumentDetail struct {
	Document
	Tags []Tag `json:"tags"`
}

// GraphNode 网络图节点
type GraphNode struct {
	ID    int64  `json:"id"`
	Label string `json:"label"`
	Size  int    `json:"size"`  // 节点大小（关联数量）
	Group int    `json:"group"` // 聚类分组
}

// GraphEdge 网络图边
type GraphEdge struct {
	From   int64 `json:"from"`
	To     int64 `json:"to"`
	Weight int   `json:"weight"` // 边粗细
}

// GraphData 完整的网络图数据
type GraphData struct {
	Nodes []GraphNode `json:"nodes"`
	Edges []GraphEdge `json:"edges"`
}

// KeywordSuggestion 关键词/标签建议（预留）
type KeywordSuggestion struct {
	Keyword string  `json:"keyword"`
	Score   float64 `json:"score"`
	Source  string  `json:"source"` // "manual" / "auto_filename" / "auto_content"
}
