# PDF 知识库管理工具 — 设计文档

**日期：** 2025-06-14
**状态：** 已确认

---

## 1. 目标

为 Windows 公司电脑构建一个 **免安装、单文件 .exe** 的 PDF 知识库管理工具。核心功能是给 PDF 文件打标签，并通过知识网络图谱可视化文档和标签之间的关联关系。

## 2. 约束条件

- **免安装**：单个 .exe 文件，放在文件夹里双击即可运行
- **零依赖**：目标机器不需要预装任何运行时（Python、Node.js 等）
- **体积小**：约 20MB
- **不移动文件**：PDF 文件保留在原始位置，软件只做索引和标签管理
- **不提取 PDF 内容**：本次不做 PDF 文本提取和关键词推荐，但预留接口

## 3. 技术选型

| 项目 | 选择 | 理由 |
|------|------|------|
| 后端语言 | **Go** | 编译为原生单文件 .exe，零依赖，启动快 |
| 桌面框架 | **Wails v2** | Go + 嵌入式 WebView2，使用 Windows 10/11 自带 Edge 内核 |
| 前端 | **HTML + JS + CSS** | 与 Go 后端通过 Wails 绑定通信 |
| 数据库 | **SQLite** | 嵌入式，零配置，数据存在用户目录下 |
| 网络图 | **vis.js** | 轻量交互式网络图库，支持拖拽/缩放/点击 |
| 打包 | **Wails build** | 前端资源内嵌到 Go 二进制，输出单文件 .exe |

## 4. 功能清单

### 4.1 本次实现

| 功能 | 说明 |
|------|------|
| 扫描文件夹 | 选择文件夹，递归扫描其中的 PDF 文件并建立索引 |
| 手动打标签 | 选中 PDF 文件，输入标签名（支持已有标签自动补全） |
| 批量打标签 | 多选 PDF 文件，一次性应用相同标签 |
| 标签筛选 | 左侧标签栏，点击标签过滤出对应的 PDF 列表 |
| 搜索 | 按文件名和标签名搜索 |
| PDF 关联网络 | 节点=PDF，边=共享标签的文档对，边粗=共享标签数 |
| 标签关联网络 | 节点=标签，边=共现在同一文档的标签对，边粗=共现次数 |
| 去掉标签 | 从文档上移除指定标签 |
| 删除标签 | 全局删除一个标签（所有文档上的该标签都移除） |
| 网络图交互 | 点击节点查看详情、搜索定位、聚类着色 |

### 4.2 预留接口（本次不实现）

| 功能 | 预留方式 |
|------|---------|
| PDF 文本提取 | `Extractor` 接口定义，当前 noop 实现 |
| 从内容推荐关键词 | `SuggestFromContent()` 方法，当前返回空 |
| PDF 内容搜索 | `documents.content` 字段预留，当前为空 |

## 5. 数据模型

```sql
-- PDF 文件记录
CREATE TABLE documents (
    id         INTEGER PRIMARY KEY,
    path       TEXT UNIQUE NOT NULL,    -- 文件绝对路径
    filename   TEXT NOT NULL,           -- 文件名
    title      TEXT,                    -- PDF 元数据标题（预留）
    content    TEXT,                    -- 提取的纯文本（预留，当前为空）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    indexed_at DATETIME                -- 最后扫描/索引时间
);

-- 标签
CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL           -- 标签名
);

-- 文档-标签 关联（多对多）
CREATE TABLE document_tags (
    document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    tag_id      INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    source      TEXT DEFAULT 'manual',  -- manual / auto_content(预留)
    PRIMARY KEY (document_id, tag_id)
);

-- 标签之间的手动关联（可选，用于强化知识关系）
CREATE TABLE tag_relations (
    tag_a_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    weight   INTEGER DEFAULT 1,
    PRIMARY KEY (tag_a_id, tag_b_id)
);
```

## 6. 后端架构

```
internal/
├── database/
│   ├── db.go          — SQLite 初始化、迁移
│   └── models.go      — 数据结构定义
├── scanner/
│   └── scanner.go     — 递归扫描文件夹中的 PDF
├── extractor/
│   ├── extractor.go   — Extractor 接口定义（预留）
│   └── noop.go        — 空实现
├── tagger/
│   └── tagger.go      — 标签 CRUD、批量操作
└── graph/
    └── graph.go       — 生成网络图数据（节点列表+边列表）
```

### 6.1 关键接口定义

```go
// extractor.go — 预留接口
package extractor

type KeywordSuggestion struct {
    Keyword string
    Score   float64
    Source  string  // "content" (预留)
}

type Extractor interface {
    ExtractText(filePath string) (string, error)
    SuggestFromContent(filePath string) ([]KeywordSuggestion, error)
}

// noop.go — 当前实现
type NoopExtractor struct{}

func (n *NoopExtractor) ExtractText(filePath string) (string, error) {
    return "", nil
}

func (n *NoopExtractor) SuggestFromContent(filePath string) ([]KeywordSuggestion, error) {
    return nil, nil
}
```

### 6.2 Wails 绑定暴露给前端的方法

```go
// App 结构体方法，前端 JS 可直接调用
type App struct {
    db        *database.DB
    scanner   *scanner.Scanner
    extractor extractor.Extractor
    tagger    *tagger.Tagger
    graph     *graph.GraphBuilder
}

// 扫描
func (a *App) ScanFolder(folderPath string) ([]Document, error)

// 文档
func (a *App) ListDocuments(tagFilter []string, searchText string) ([]Document, error)
func (a *App) GetDocument(id int) (*DocumentDetail, error)

// 标签
func (a *App) ListTags() ([]TagWithCount, error)
func (a *App) AddTagToDocument(docID int, tagName string) error
func (a *App) RemoveTagFromDocument(docID int, tagID int) error
func (a *App) BatchAddTag(docIDs []int, tagName string) error
func (a *App) DeleteTag(tagID int) error
func (a *App) RenameTag(tagID int, newName string) error
func (a *App) AutoSuggestTags() []KeywordSuggestion  // 预留，当前返回空

// 网络图
func (a *App) GetDocumentGraph() (*GraphData, error)
func (a *App) GetTagGraph() (*GraphData, error)

// 系统
func (a *App) OpenFile(docID int) error       // 用系统默认程序打开 PDF
func (a *App) OpenFileLocation(docID int) error // 打开文件所在目录
func (a *App) SelectFolder() (string, error)   // 弹出文件夹选择对话框
```

## 7. 前端 UI

### 7.1 布局

```
┌──────────────────────────────────────────────────────┐
│  🔍 搜索框          │ [扫描文件夹] [网络图谱] [设置]  │
├─────────┬─────────────────────────┬─────────────────┤
│ 标签筛选 │  PDF 文件列表            │  详情面板       │
│         │                        │                │
│ #机器学习│  📄 论文A.pdf           │  文件名         │
│   (3)   │    标签: 机器学习 深度学习│  标签: [x] [x]  │
│ #深度学习│                        │  [输入框+补全]   │
│   (2)   │  📄 论文B.pdf           │                │
│ #NLP    │    标签: 机器学习 NLP    │  [用系统打开]    │
│   (1)   │                        │  [打开目录]      │
│ #项目管理│  📄 技术报告.pdf         │                │
│   (1)   │    标签: 项目管理       │                │
│         │                        │                │
├─────────┴─────────────────────────┴─────────────────┤
│              知识网络图谱（全屏模式可用）                │
│            vis.js 交互式网络图                         │
│  [切换: PDF关联网络 | 标签关联网络]  [搜索节点]         │
└──────────────────────────────────────────────────────┘
```

### 7.2 网络图交互

- **节点**：圆形，大小按关联数量缩放
- **边**：粗细按共享标签数/共现次数缩放
- **聚类着色**：关联紧密的节点群用同一种颜色
- **点击**：节点弹出详情气泡（文件名/标签名 + 关联列表）
- **搜索**：输入关键词高亮匹配的节点
- **切换**：顶部按钮切换 PDF 关联视图 / 标签关联视图

## 8. 项目文件结构

```
pdf-knowledge-base/
├── main.go                      # Wails 应用入口
├── wails.json                   # Wails 配置
├── go.mod
├── go.sum
├── internal/
│   ├── app.go                   # App 结构体 + Wails 绑定方法
│   ├── database/
│   │   ├── db.go                # SQLite 连接、建表迁移
│   │   └── models.go            # 数据结构定义
│   ├── scanner/
│   │   └── scanner.go           # 递归扫描 PDF
│   ├── extractor/
│   │   ├── extractor.go         # Extractor 接口定义
│   │   └── noop.go              # 空实现
│   ├── tagger/
│   │   └── tagger.go            # 标签 CRUD + 批量
│   └── graph/
│       └── graph.go             # 网络图数据生成
├── frontend/
│   ├── index.html               # 主页面
│   ├── package.json
│   ├── src/
│   │   ├── main.js              # 应用入口
│   │   ├── style.css            # 全局样式
│   │   ├── components/
│   │   │   ├── FileList.js      # PDF 文件列表组件
│   │   │   ├── TagPanel.js      # 标签筛选面板
│   │   │   ├── DetailPanel.js   # PDF 详情 + 标签编辑
│   │   │   ├── SearchBar.js     # 搜索栏
│   │   │   └── NetworkGraph.js  # vis.js 网络图谱
│   │   └── api/
│   │       └── bindings.js      # Wails 自动生成的 Go 方法绑定
│   └── wailsjs/                 # Wails 自动生成目录
├── docs/
│   └── superpowers/
│       └── specs/
│           └── 2025-06-14-pdf-knowledge-base-design.md
└── build/
    └── appicon.png
```

## 9. 数据存储位置

```
%APPDATA%/pdf-knowledge-base/
├── data.db                      # SQLite 数据库
└── config.json                  # 用户配置（扫描的文件夹列表等）
```

## 10. 开发计划概要

1. **阶段一**：项目初始化 — Wails 脚手架 + SQLite 建表
2. **阶段二**：后端核心 — 扫描器 + 标签 CRUD + 批量操作
3. **阶段三**：前端 UI — 文件列表 + 标签面板 + 详情编辑 + 搜索
4. **阶段四**：网络图谱 — vis.js 集成 + 双视图切换
5. **阶段五**：打磨 — 标签补全、多选批量、打开文件、打包发布
