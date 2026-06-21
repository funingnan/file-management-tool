package tagger

import "pdf-knowledge-base/internal/database"

// Tagger 标签管理器
type Tagger struct {
	db *database.DB
}

// New 创建标签管理器
func New(db *database.DB) *Tagger {
	return &Tagger{db: db}
}

// AddTagToDocument 给文档添加标签
func (t *Tagger) AddTagToDocument(docID int64, tagName string) error {
	tagID, err := t.db.EnsureTag(tagName)
	if err != nil {
		return err
	}
	return t.db.AddTagToDocument(docID, tagID, "manual")
}

// RemoveTagFromDocument 从文档移除标签
func (t *Tagger) RemoveTagFromDocument(docID, tagID int64) error {
	return t.db.RemoveTagFromDocument(docID, tagID)
}

// BatchAddTag 给多个文档批量添加同一标签
func (t *Tagger) BatchAddTag(docIDs []int64, tagName string) (int, error) {
	tagID, err := t.db.EnsureTag(tagName)
	if err != nil {
		return 0, err
	}
	count := 0
	for _, docID := range docIDs {
		if err := t.db.AddTagToDocument(docID, tagID, "manual"); err == nil {
			count++
		}
	}
	return count, nil
}

// DeleteTag 全局删除标签
func (t *Tagger) DeleteTag(tagID int64) error {
	return t.db.DeleteTag(tagID)
}

// RenameTag 重命名标签
func (t *Tagger) RenameTag(tagID int64, newName string) error {
	return t.db.RenameTag(tagID, newName)
}
