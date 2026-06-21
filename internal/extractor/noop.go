package extractor

import "pdf-knowledge-base/internal/database"

// NoopExtractor 空实现，当前不提取 PDF 内容
type NoopExtractor struct{}

// NewNoop 创建空提取器
func NewNoop() *NoopExtractor {
	return &NoopExtractor{}
}

// ExtractText 预留接口，当前返回空字符串
func (n *NoopExtractor) ExtractText(filePath string) (string, error) {
	return "", nil
}

// SuggestFromContent 预留接口，当前返回空列表
func (n *NoopExtractor) SuggestFromContent(filePath string) ([]database.KeywordSuggestion, error) {
	return nil, nil
}
