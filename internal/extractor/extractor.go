package extractor

import "pdf-knowledge-base/internal/database"

// Extractor PDF 内容提取接口（预留）
// 未来可实现 PDF 文本提取和关键词推荐
type Extractor interface {
	// ExtractText 提取 PDF 的纯文本内容
	ExtractText(filePath string) (string, error)
	// SuggestFromContent 从 PDF 内容推荐关键词标签
	SuggestFromContent(filePath string) ([]database.KeywordSuggestion, error)
}
