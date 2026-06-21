package scanner

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// setupTestDir 创建一个包含 N 个模拟文件的临时目录
func setupTestDir(b *testing.B, fileCount int) string {
	b.Helper()
	dir := b.TempDir()
	// 创建一些子目录以模拟真实目录结构
	subDirs := []string{"docs", "images", "videos", "sub/docs", "sub/images"}
	for _, sd := range subDirs {
		os.MkdirAll(filepath.Join(dir, sd), 0755)
	}

	exts := []string{".pdf", ".docx", ".jpg", ".png", ".mp4", ".txt"}
	for i := 0; i < fileCount; i++ {
		ext := exts[i%len(exts)]
		subDir := subDirs[i%len(subDirs)]
		name := filepath.Join(dir, subDir, fmt.Sprintf("file_%05d%s", i, ext))
		os.WriteFile(name, []byte("test"), 0644)
	}
	return dir
}

func BenchmarkScanFolder_100(b *testing.B) {
	dir := setupTestDir(b, 100)
	enabledTypes := []string{"pdf", "docx", "image", "video"}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = ScanFolder(dir, enabledTypes)
	}
}

func BenchmarkScanFolder_1000(b *testing.B) {
	dir := setupTestDir(b, 1000)
	enabledTypes := []string{"pdf", "docx", "image", "video"}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = ScanFolder(dir, enabledTypes)
	}
}

func BenchmarkScanFolder_3000(b *testing.B) {
	dir := setupTestDir(b, 3000)
	enabledTypes := []string{"pdf", "docx", "image", "video"}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = ScanFolder(dir, enabledTypes)
	}
}
