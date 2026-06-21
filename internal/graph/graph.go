package graph

import "pdf-knowledge-base/internal/database"

// GraphBuilder 网络图数据构建器
type GraphBuilder struct {
	db *database.DB
}

// New 创建网络图构建器
func New(db *database.DB) *GraphBuilder {
	return &GraphBuilder{db: db}
}

// BuildDocumentGraph 构建 PDF 关联网络
// 节点 = PDF 文件，边 = 共享标签
func (g *GraphBuilder) BuildDocumentGraph() (*database.GraphData, error) {
	return g.db.GetDocumentGraph()
}

// BuildTagGraph 构建标签关联网络
// 节点 = 标签，边 = 共现在同一文档
func (g *GraphBuilder) BuildTagGraph() (*database.GraphData, error) {
	return g.db.GetTagGraph()
}
