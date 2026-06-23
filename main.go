package main

import (
	"log"
	"os"
	"path/filepath"

	"pdf-knowledge-base/internal/database"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
)

func main() {
	// 数据存储在可执行文件同目录下
	execPath, err := os.Executable()
	if err != nil {
		log.Fatal("无法获取程序路径:", err)
	}
	execDir := filepath.Dir(execPath)
	dataDir := filepath.Join(execDir, "data")

	// 免安装 WebView2 运行时支持
	// 如果 exe 同目录下存在 WebView2 文件夹，则使用它
	webview2Dir := filepath.Join(execDir, "WebView2")
	if info, err := os.Stat(webview2Dir); err == nil && info.IsDir() {
		os.Setenv("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", webview2Dir)
	}

	// 初始化数据库
	db, err := database.New(dataDir)
	if err != nil {
		log.Fatal("数据库初始化失败:", err)
	}
	defer db.Close()

	// 创建应用
	app := NewApp(db, dataDir)

	// 启动 Wails
	err = wails.Run(&options.App{
		Title:  "PDF 知识库",
		Width:  1180,
		Height: 800,
		MinWidth: 1196,
		MinHeight: 600,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 245, G: 245, B: 245, A: 1},
		OnStartup:        app.Startup,
		Bind: []interface{}{
			app,
		},
	})

	if err != nil {
		log.Fatal("应用启动失败:", err)
	}
}
