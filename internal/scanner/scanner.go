package scanner

import (
	"io/fs"
	"path/filepath"
	"strings"
	"time"
)

// FileTypeConfig 文件类型配置
type FileTypeConfig struct {
	Ext      string // 扩展名如 ".pdf"
	Type     string // 类型标识如 "pdf"
	Category string // 分类: "document" / "image" / "video"
}

// 所有支持的文件类型
var allFileTypes = []FileTypeConfig{
	// 文档
	{".pdf", "pdf", "document"},
	{".doc", "docx", "document"},
	{".docx", "docx", "document"},
	{".xls", "xlsx", "document"},
	{".xlsx", "xlsx", "document"},
	{".ppt", "pptx", "document"},
	{".pptx", "pptx", "document"},
	// 图片
	{".jpg", "image", "image"},
	{".jpeg", "image", "image"},
	{".png", "image", "image"},
	{".gif", "image", "image"},
	{".bmp", "image", "image"},
	{".webp", "image", "image"},
	{".svg", "image", "image"},
	{".ico", "image", "image"},
	{".tiff", "image", "image"},
	{".tif", "image", "image"},
	// 视频
	{".mp4", "video", "video"},
	{".avi", "video", "video"},
	{".mkv", "video", "video"},
	{".mov", "video", "video"},
	{".wmv", "video", "video"},
	{".flv", "video", "video"},
	{".webm", "video", "video"},
	{".m4v", "video", "video"},
}

// 根据启用的分类构建扩展名查找表
func buildExtMap(enabledTypes []string) map[string]string {
	extMap := make(map[string]string)
	for _, ft := range allFileTypes {
		for _, enabled := range enabledTypes {
			if ft.Type == enabled {
				extMap[ft.Ext] = ft.Type
				break
			}
		}
	}
	return extMap
}

// ScanResult 扫描结果
type ScanResult struct {
	Path     string
	Filename string
	FileType string
	FileSize int64
	ModTime  time.Time
}

// ScanFolder 递归扫描文件夹中支持的文件类型
// enabledTypes: 启用的类型列表如 ["pdf","docx","xlsx","pptx","image","video"]
func ScanFolder(folderPath string, enabledTypes []string) ([]ScanResult, error) {
	extMap := buildExtMap(enabledTypes)
	var results []ScanResult

	// 预先将folderPath转为绝对路径，避免循环内重复调用filepath.Abs
	absFolderPath, err := filepath.Abs(folderPath)
	if err != nil {
		absFolderPath = folderPath
	}

	err = filepath.WalkDir(absFolderPath, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() {
			return nil
		}

		// 只对匹配扩展名的文件获取详细信息
		ext := strings.ToLower(filepath.Ext(d.Name()))
		fileType, ok := extMap[ext]
		if !ok {
			return nil
		}

		// 获取文件信息（只对匹配的文件调用Info()）
		info, err := d.Info()
		if err != nil {
			return nil
		}

		results = append(results, ScanResult{
			Path:     path, // WalkDir返回的路径已经是绝对路径
			Filename: d.Name(),
			FileType: fileType,
			FileSize: info.Size(),
			ModTime:  info.ModTime(),
		})
		return nil
	})

	return results, err
}

// GetSupportedTypes 返回所有支持的文件类型标识（去重）
func GetSupportedTypes() []string {
	seen := make(map[string]bool)
	var types []string
	for _, ft := range allFileTypes {
		if !seen[ft.Type] {
			seen[ft.Type] = true
			types = append(types, ft.Type)
		}
	}
	return types
}
