package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"

	"pdf-knowledge-base/internal/database"
	"pdf-knowledge-base/internal/extractor"
	"pdf-knowledge-base/internal/graph"
	"pdf-knowledge-base/internal/scanner"
	"pdf-knowledge-base/internal/tagger"

	wailsRuntime "github.com/wailsapp/wails/v2/pkg/runtime"
)

// App Wails 应用主结构体，所有公开方法都可通过前端 JS 调用
type App struct {
	ctx       context.Context
	db        *database.DB
	extractor extractor.Extractor
	tagger    *tagger.Tagger
	graph     *graph.GraphBuilder
	mode      string // "app" (默认) 或 "tag-picker"
	filePath  string // tag-picker 模式下的目标文件路径
	dataDir   string // 数据目录路径
}

// NewApp 创建应用实例
func NewApp(db *database.DB, dataDir string) *App {
	return &App{
		db:        db,
		extractor: extractor.NewNoop(),
		tagger:    tagger.New(db),
		graph:     graph.New(db),
		mode:      "app",
		dataDir:   dataDir,
	}
}

// Startup Wails 启动时调用
func (a *App) Startup(ctx context.Context) {
	a.ctx = ctx
}

// ---------- 标签选择器模式 ----------

// SetTagPickerMode 设置为标签选择器模式
func (a *App) SetTagPickerMode(filePath string) {
	a.mode = "tag-picker"
	a.filePath = filePath
}

// GetAppMode 获取当前应用模式（前端调用）
func (a *App) GetAppMode() map[string]interface{} {
	return map[string]interface{}{
		"mode":     a.mode,
		"filePath": a.filePath,
	}
}

// AddTagToFilePath 通过文件路径给文档添加标签（右键菜单调用）
func (a *App) AddTagToFilePath(filePath string, tagName string) error {
	// 先尝试查找已有文档
	doc, err := a.db.GetDocumentByPath(filePath)
	if err != nil {
		// 文档不在数据库中，自动入库
		doc, err = a.db.UpsertDocumentFromPath(filePath)
		if err != nil {
			return fmt.Errorf("文件入库失败: %w", err)
		}
	}
	return a.tagger.AddTagToDocument(doc.ID, tagName)
}

// RemoveTagFromFilePath 通过文件路径移除文档标签
func (a *App) RemoveTagFromFilePath(filePath string, tagID int64) error {
	doc, err := a.db.GetDocumentByPath(filePath)
	if err != nil {
		return err
	}
	return a.tagger.RemoveTagFromDocument(doc.ID, tagID)
}

// GetDocumentTagsByPath 通过文件路径获取文档当前标签
func (a *App) GetDocumentTagsByPath(filePath string) ([]database.Tag, error) {
	doc, err := a.db.GetDocumentByPath(filePath)
	if err != nil {
		// 文档不在数据库中，返回空标签
		return nil, nil
	}
	detail, err := a.db.GetDocument(doc.ID)
	if err != nil {
		return nil, err
	}
	return detail.Tags, nil
}

// CloseTagPicker 关闭标签选择器窗口
func (a *App) CloseTagPicker() {
	wailsRuntime.Quit(a.ctx)
}

// ---------- 扫描功能 ----------

// ScanFolderResult 扫描结果
type ScanFolderResult struct {
	NewFiles   int `json:"newFiles"`
	Total      int `json:"total"`
	Relocated  int `json:"relocated"` // 智能匹配找回的文件数
}

// ScanFolder 扫描指定文件夹，将文件索引到数据库
func (a *App) ScanFolder(folderPath string, enabledTypes []string) (*ScanFolderResult, error) {
	results, err := scanner.ScanFolder(folderPath, enabledTypes)
	if err != nil {
		return nil, fmt.Errorf("扫描失败: %w", err)
	}

	// 转换为批量插入格式
	docs := make([]database.DocumentInput, len(results))
	for i, r := range results {
		docs[i] = database.DocumentInput{
			Path:     r.Path,
			Filename: r.Filename,
			Title:    "",
			FileType: r.FileType,
			FileSize: r.FileSize,
			ModTime:  r.ModTime,
		}
	}

	// 批量写入数据库（单事务）
	newCount, err := a.db.UpsertDocuments(docs)
	if err != nil {
		return nil, fmt.Errorf("批量写入数据库失败: %w", err)
	}

	// 智能匹配：尝试找回改名/移动的文件
	relocated := a.smartRelocate(folderPath, results)

	return &ScanFolderResult{
		NewFiles:  newCount,
		Total:     len(results),
		Relocated: relocated,
	}, nil
}

// smartRelocate 用文件大小+修改时间匹配丢失的文件
func (a *App) smartRelocate(folderPath string, scanned []scanner.ScanResult) int {
	// 如果扫描结果数量大于等于数据库文档数量，说明没有丢失的文件，跳过智能匹配
	dbCount, err := a.db.CountDocuments()
	if err != nil || len(scanned) >= dbCount {
		return 0
	}

	lost, err := a.db.ListLostDocuments(folderPath)
	if err != nil || len(lost) == 0 {
		return 0
	}

	// 建立扫描结果的指纹索引: "size:modTimeSec" → ScanResult
	type fingerprint struct {
		size    int64
		modSec  int64
	}
	fingerprints := make(map[fingerprint]scanner.ScanResult)
	for _, s := range scanned {
		fp := fingerprint{size: s.FileSize, modSec: s.ModTime.Unix()}
		fingerprints[fp] = s
	}

	relocated := 0
	for _, doc := range lost {
		fp := fingerprint{size: doc.FileSize, modSec: doc.ModTime.Unix()}
		if match, ok := fingerprints[fp]; ok {
			// 找到匹配！更新路径，保留标签
			if err := a.db.UpdateDocumentPath(doc.ID, match.Path, match.Filename, match.FileSize, match.ModTime); err == nil {
				relocated++
			}
		}
	}
	return relocated
}

// SelectFolder 弹出文件夹选择对话框
func (a *App) SelectFolder() (string, error) {
	folder, err := wailsRuntime.OpenDirectoryDialog(a.ctx, wailsRuntime.OpenDialogOptions{
		Title: "选择文件夹",
	})
	if err != nil {
		return "", err
	}
	return folder, nil
}

// ---------- 文档操作 ----------

// ListDocuments 查询文档列表
func (a *App) ListDocuments(tagIDs []int64, searchText string, untagged bool, fileTypes []string) ([]database.Document, error) {
	return a.db.ListDocuments(tagIDs, searchText, untagged, fileTypes)
}

// GetDocument 获取文档详情（含标签）
func (a *App) GetDocument(id int64) (*database.DocumentDetail, error) {
	return a.db.GetDocument(id)
}

// GetDocumentCount 获取文档总数
func (a *App) GetDocumentCount() (int, error) {
	return a.db.CountDocuments()
}

// GetUntaggedCount 获取无标签文档数
func (a *App) GetUntaggedCount(fileTypes []string) (int, error) {
	return a.db.CountUntaggedDocuments(fileTypes)
}

// RemoveDocument 从列表移除文档（不删真实文件）
func (a *App) RemoveDocument(docID int64) error {
	return a.db.RemoveDocument(docID)
}

// RemoveDocuments 批量移除文档
func (a *App) RemoveDocuments(docIDs []int64) (int, error) {
	count := 0
	for _, id := range docIDs {
		if err := a.db.RemoveDocument(id); err == nil {
			count++
		}
	}
	return count, nil
}

// GetFileTypeCounts 获取各文件类型数量
func (a *App) GetFileTypeCounts() (map[string]int, error) {
	return a.db.CountByFileType()
}

// GetSupportedTypes 获取所有支持的文件类型
func (a *App) GetSupportedTypes() []string {
	return scanner.GetSupportedTypes()
}

// ---------- 标签操作 ----------

// ListTags 获取所有标签（含使用次数）
func (a *App) ListTags() ([]database.TagWithCount, error) {
	return a.db.ListTags()
}

// AddTagToDocument 给文档添加标签
func (a *App) AddTagToDocument(docID int64, tagName string) error {
	return a.tagger.AddTagToDocument(docID, tagName)
}

// RemoveTagFromDocument 从文档移除标签
func (a *App) RemoveTagFromDocument(docID int64, tagID int64) error {
	return a.tagger.RemoveTagFromDocument(docID, tagID)
}

// BatchRemoveTagFromDocuments 批量移除多个文档的指定标签
func (a *App) BatchRemoveTagFromDocuments(docIDs []int64, tagID int64) (int, error) {
	count := 0
	for _, docID := range docIDs {
		if err := a.tagger.RemoveTagFromDocument(docID, tagID); err == nil {
			count++
		}
	}
	return count, nil
}

// BatchAddTag 给多个文档批量添加标签
func (a *App) BatchAddTag(docIDs []int64, tagName string) (int, error) {
	return a.tagger.BatchAddTag(docIDs, tagName)
}

// DeleteTag 全局删除标签
func (a *App) DeleteTag(tagID int64) error {
	return a.tagger.DeleteTag(tagID)
}

// RenameTag 重命名标签
func (a *App) RenameTag(tagID int64, newName string) error {
	return a.tagger.RenameTag(tagID, newName)
}

// ---------- 系统操作 ----------

// OpenFile 用系统默认程序打开 PDF 文件
func (a *App) OpenFile(docID int64) error {
	doc, err := a.db.GetDocument(docID)
	if err != nil {
		return err
	}
	return openPath(doc.Path)
}

// OpenFileLocation 打开文件所在目录
func (a *App) OpenFileLocation(docID int64) error {
	doc, err := a.db.GetDocument(docID)
	if err != nil {
		return err
	}
	return openDir(doc.Path)
}

// ---------- 设置 ----------

// Settings 应用设置
type Settings struct {
	EnabledTypes     []string `json:"enabledTypes"`      // 启用的文件类型
	CurrentFolderPath string   `json:"currentFolderPath"` // 当前选择的文件夹路径
}

// 默认设置
var defaultSettings = Settings{
	EnabledTypes:     []string{"pdf", "docx", "xlsx", "pptx", "image", "video"},
	CurrentFolderPath: "",
}

// GetSettings 获取当前设置
func (a *App) GetSettings() (*Settings, error) {
	return loadSettings()
}

// SaveSettings 保存设置
func (a *App) SaveSettings(s *Settings) error {
	return saveSettings(s)
}

func settingsPath() string {
	exePath, err := os.Executable()
	if err != nil {
		return "settings.json"
	}
	return filepath.Join(filepath.Dir(exePath), "data", "settings.json")
}

func loadSettings() (*Settings, error) {
	data, err := os.ReadFile(settingsPath())
	if err != nil {
		// 返回默认设置
		s := defaultSettings
		return &s, nil
	}
	var s Settings
	if err := json.Unmarshal(data, &s); err != nil {
		s2 := defaultSettings
		return &s2, nil
	}
	if len(s.EnabledTypes) == 0 {
		s.EnabledTypes = defaultSettings.EnabledTypes
	}
	return &s, nil
}

func saveSettings(s *Settings) error {
	path := settingsPath()
	os.MkdirAll(filepath.Dir(path), 0755)
	data, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

// ---------- 网络图 ----------

// GetDocumentGraph 获取 PDF 关联网络图数据
func (a *App) GetDocumentGraph() (*database.GraphData, error) {
	return a.graph.BuildDocumentGraph()
}

// GetTagGraph 获取标签关联网络图数据
func (a *App) GetTagGraph() (*database.GraphData, error) {
	return a.graph.BuildTagGraph()
}

// ---------- 数据导入导出 ----------

// ExportDatabase 导出数据库文件，返回保存路径
func (a *App) ExportDatabase() (string, error) {
	// WAL checkpoint 确保数据完整
	_, err := a.db.Exec("PRAGMA wal_checkpoint(TRUNCATE)")
	if err != nil {
		return "", fmt.Errorf("数据库检查点失败: %w", err)
	}

	srcPath := filepath.Join(a.dataDir, "data.db")
	savePath, err := wailsRuntime.SaveFileDialog(a.ctx, wailsRuntime.SaveDialogOptions{
		Title:           "导出数据库",
		DefaultFilename: "data.db",
		Filters: []wailsRuntime.FileFilter{
			{DisplayName: "数据库文件", Pattern: "*.db"},
		},
	})
	if err != nil {
		return "", err
	}
	if savePath == "" {
		return "", nil // 用户取消
	}

	srcData, err := os.ReadFile(srcPath)
	if err != nil {
		return "", fmt.Errorf("读取数据库失败: %w", err)
	}
	if err := os.WriteFile(savePath, srcData, 0644); err != nil {
		return "", fmt.Errorf("写入文件失败: %w", err)
	}

	return savePath, nil
}

// ImportDatabase 导入数据库文件，替换当前数据库。返回是否实际执行了导入
func (a *App) ImportDatabase() (bool, error) {
	openPath, err := wailsRuntime.OpenFileDialog(a.ctx, wailsRuntime.OpenDialogOptions{
		Title: "选择要导入的数据库文件",
		Filters: []wailsRuntime.FileFilter{
			{DisplayName: "数据库文件", Pattern: "*.db"},
		},
	})
	if err != nil {
		return false, err
	}
	if openPath == "" {
		return false, nil
	}

	// 验证文件是否为合法 SQLite
	srcData, err := os.ReadFile(openPath)
	if err != nil {
		return false, fmt.Errorf("读取文件失败: %w", err)
	}
	if len(srcData) < 16 || string(srcData[:16])[:15] != "SQLite format 3" {
		return false, fmt.Errorf("不是有效的数据库文件")
	}

	// 关闭现有连接
	if err := a.db.Close(); err != nil {
		return false, fmt.Errorf("关闭数据库失败: %w", err)
	}

	// 替换文件
	dstPath := filepath.Join(a.dataDir, "data.db")
	if err := os.WriteFile(dstPath, srcData, 0644); err != nil {
		return false, fmt.Errorf("写入数据库失败: %w", err)
	}

	// 清理 WAL/SHM 文件
	os.Remove(dstPath + "-wal")
	os.Remove(dstPath + "-shm")

	// 重新打开数据库
	newDB, err := database.New(a.dataDir)
	if err != nil {
		return false, fmt.Errorf("重新打开数据库失败: %w", err)
	}
	a.db = newDB
	a.tagger = tagger.New(newDB)
	a.graph = graph.New(newDB)

	return true, nil
}

// ---------- 工具函数 ----------

func openPath(path string) error {
	switch runtime.GOOS {
	case "windows":
		return exec.Command("cmd", "/c", "start", "", path).Start()
	case "darwin":
		return exec.Command("open", path).Start()
	default:
		return exec.Command("xdg-open", path).Start()
	}
}

func openDir(path string) error {
	switch runtime.GOOS {
	case "windows":
		return exec.Command("explorer", "/select,", path).Start()
	case "darwin":
		return exec.Command("open", "-R", path).Start()
	default:
		return exec.Command("xdg-open", filepath.Dir(path)).Start()
	}
}
