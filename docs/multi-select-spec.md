# 多选框、文件选择、详情面板与批量操作 — 功能规格文档

> 本文档用于 Rust 重写，详细描述触发区域、状态管理、UI 布局、功能逻辑。

---

## 一、整体布局（三栏）

```
┌──────────────────────────────────────────────────────────────────┐
│  #toolbar (高度 50px, flex-shrink:0)                              │
│  [扫描文件] [选择路径]    [搜索框]     [列表图标][图谱图标] 0个文件 │
├────────┬─────────────────────────────┬───────────────────────────┤
│        │                             │                           │
│  左栏  │        中栏                 │         右栏              │
│ 220px  │        flex:1               │        300px              │
│        │                             │                           │
│ 文件类型│  #file-list-header (56px)   │  #detail-panel            │
│ 筛选栏  │  [全选][已选数][操作按钮]   │  文件详情/标签管理        │
│        │  ─────────────────────────  │                           │
│ 标签    │  #file-list (可滚动)       │                           │
│ 列表    │  文件项列表                 │                           │
│        │                             │                           │
│ [设置]  │                             │                           │
└────────┴─────────────────────────────┴───────────────────────────┘
```

HTML 结构：
```html
<body>
  <header id="toolbar">...</header>
  <div id="app-wrapper">            <!-- position:relative, flex:1 -->
    <div id="main-content">          <!-- display:flex, flex:1 -->
      <aside id="tag-panel">...</aside>     <!-- width:220px -->
      <section id="file-panel">...</section> <!-- flex:1 -->
      <aside id="detail-panel">...</aside>   <!-- width:300px -->
    </div>
    <div id="graph-view">...</div>   <!-- position:absolute, z-index:50 -->
  </div>
</body>
```

---

## 二、状态定义

```typescript
interface State {
    // 文件列表
    documents: Document[];           // 当前显示的文件列表
    selectedDocId: number | null;    // 单选：当前查看详情的文件 ID

    // 多选
    selectedDocIds: Set<number>;     // 复选框选中的文件 ID
    multiSelectedIds: Set<number>;   // 多选集合（与 selectedDocIds 始终同步）
    lastClickedIndex: number;        // 上次点击索引（预留 Shift 多选）

    // 标签
    allTags: Tag[];                  // 全部标签
    tagCache: Record<number, string>; // docId → 标签 HTML 缓存

    // 筛选
    activeTagIds: number[];          // 当前筛选的标签 ID
    filterMode: string;              // 'all' | 'folder' | 'untagged'
    fileTypeFilter: string;          // 'all' | 'pdf' | 'docx' | ...
    currentFolderPath: string;       // 文件夹模式下的路径
    searchText: string;              // 搜索关键词

    // 视图
    viewMode: string;                // 'list' | 'graph'
    graphMode: string;               // 'document' | 'tag'
    graphNetwork: any;               // vis.Network 实例

    // 设置
    settings: { enabledTypes: string[] };
}
```

### 三个选中状态的关系

| 字段 | 用途 | 控制的 UI |
|------|------|-----------|
| `selectedDocId` | 单选，查看右侧详情面板 | 右栏详情 + 文件列表 active 高亮 |
| `selectedDocIds` | 复选框 checked 状态 | 文件项复选框 ☑ |
| `multiSelectedIds` | 批量操作数据源 | 批量操作栏计数 + 传给后端 API |

**关键约束**：`selectedDocIds` 和 `multiSelectedIds` 在所有操作中必须保持同步。

---

## 三、中栏 — 文件列表区

### 3.1 文件列表头部 `#file-list-header`

```
┌──────────────────────────────────────────────────────────────────┐
│ [☑ 全选] │ 0 个已选 │ [取消选择] [移除文件] [标签输入框] [添加标签] [移除标签] │
└──────────────────────────────────────────────────────────────────┘
```

**尺寸**：高度 56px，padding: 0 14px

**HTML**：
```html
<div id="file-list-header">
    <label id="select-all-wrap">
        <input type="checkbox" id="select-all" /> 全选
    </label>
    <div id="batch-actions">
        <span id="selected-count">0 个已选</span>
        <button id="btn-deselect-all">取消选择</button>
        <button id="btn-batch-remove-docs" class="btn-danger">移除文件</button>
        <div class="tag-pick-wrapper">
            <input type="text" id="batch-tag-input" placeholder="输入或选择标签..." />
            <div id="batch-tag-picker">...</div>
        </div>
        <button id="btn-batch-tag">添加标签</button>
        <button id="btn-batch-remove-tags" class="btn-warn">移除标签</button>
    </div>
</div>
```

**布局**：
- `#select-all-wrap`：padding-left: 13px
- `#batch-actions`：position: absolute, left: 100px, 填满剩余宽度
- `#selected-count`：width: 70px, text-align: right
- `#batch-tag-input`：width: 140px

### 3.2 文件项 `.file-item`

每个文件项分为 **两个点击区域**：

```
┌────────────────────────────────────────────────────────────┐
│ .click-toggle-zone            │ .file-info                 │
│  [☑] [📄 icon]                │  filename.txt              │
│                               │  [tag1] [tag2]             │
│  点击：切换复选框 + 查看详情   │  点击：仅查看详情          │
└───────────────────────────────┴────────────────────────────┘
```

**HTML 模板**：
```html
<div class="file-item {active} {selected} checkbox-mode" data-id="{docId}">
    <div class="click-toggle-zone">
        <input type="checkbox" {checked} />
        <span class="file-icon">{icon}</span>
    </div>
    <div class="file-info">
        <div class="file-name" title="{fullPath}">{filename}</div>
        <div class="file-tags" id="file-tags-{docId}">{tagsHtml}</div>
    </div>
</div>
```

**尺寸**：
- `.file-item`：min-height: 56px, padding: 0 12px, gap: 10px
- `.click-toggle-zone`：height: 56px, padding: 0 16px, margin: 0 -12px（扩展到行边缘）
- 复选框：16×16px, accent-color: #4a90d9

**选中状态样式**：

| 状态 | CSS 类 | 样式 |
|------|--------|------|
| 悬停 | `.file-item:hover` | background: #e8eff8 |
| 复选框选中 | `.file-item.selected` | background: #ddeeff |
| 当前查看详情 | `.file-item.active` | background: #c5dbf5, border-color: #4a90d9 |
| active + selected | 同时存在 | active 覆盖 selected |

**复选框显示控制**：
- 默认隐藏：`.file-item input[type="checkbox"] { display: none }`
- 显示：`.file-item.checkbox-mode input[type="checkbox"] { display: block }`
- 所有文件项渲染时都带 `checkbox-mode` 类

---

## 四、中栏 — 交互逻辑

### 4.1 区域 1 点击：`.click-toggle-zone`

```
点击 .click-toggle-zone
  │
  ├─ 1. 如果 e.target 是 checkbox 本身 → 不手动切换（浏览器原生处理）
  │    否则 → cb.checked = !cb.checked
  │
  ├─ 2. 更新 selectedDocIds：
  │     ├─ checked → add(docId)
  │     └─ unchecked → delete(docId)
  │
  ├─ 3. 同步 multiSelectedIds（与 selectedDocIds 一致）
  │
  ├─ 4. toggle .selected CSS 类
  │
  ├─ 5. updateBatchActions() 更新计数
  │
  └─ 6. selectDocument(docId) 更新详情面板
```

### 4.2 区域 2 点击：`.file-info`

```
点击 .file-info
  │
  └─ selectDocument(docId) — 仅查看详情，不改变选中状态
```

### 4.3 复选框 change 事件

```
checkbox change
  │
  ├─ 更新 selectedDocIds（同步逻辑同上）
  ├─ 同步 multiSelectedIds
  ├─ toggle .selected CSS 类
  └─ updateBatchActions()
```

### 4.4 全选 `#select-all`

```
#select-all change
  │
  ├─ checked = true：
  │   ├─ 清空两个集合
  │   ├─ 遍历 state.documents → 所有 id 加入两个集合
  │   └─ renderFileList()
  │
  └─ checked = false：
      ├─ 清空两个集合
      └─ renderFileList()
```

### 4.5 取消选择 `#btn-deselect-all`

```
点击
  ├─ 清空 multiSelectedIds + selectedDocIds
  ├─ 取消 select-all 复选框
  └─ renderFileList()
```

### 4.6 Shift/Ctrl 多选（当前已禁用，预留）

- **Shift + 点击**：范围选中 lastClickedIndex..currentIndex
- **Ctrl + 点击**：切换单个文件的选中状态

---

## 五、右栏 — 详情面板 `#detail-panel`

### 5.1 两种状态

| 状态 | 显示内容 |
|------|----------|
| 未选中文件 | `#detail-empty`：提示 "👈 选择一个文件查看详情" |
| 已选中文件 | `#detail-content`：文件详情 + 标签管理 |

切换方式：
```javascript
// 未选中
detail-empty.style.display = 'flex';
detail-content.style.display = 'none';

// 已选中
detail-empty.style.display = 'none';
detail-content.style.display = 'block';
```

### 5.2 详情面板布局

```
┌──────────────────────────────────────┐
│ #detail-panel (width: 300px)         │
│                                      │
│ ┌──────────────────────────────────┐ │
│ │ #detail-content                  │ │
│ │                                  │ │
│ │ 📄 filename.pdf                  │ │  ← #detail-filename (h3)
│ │ C:\Users\xxx\docs\file.pdf      │ │  ← #detail-path (p, 灰色小字)
│ │                                  │ │
│ │ [打开文件] [打开目录] [移除文件]  │ │  ← .detail-actions
│ │                                  │ │
│ │ ── 标签 ──────────────────────── │ │
│ │ [tag1 ×] [tag2 ×] [tag3 ×]      │ │  ← #detail-tags (当前标签)
│ │                                  │ │
│ │ [输入新标签...            ]       │ │  ← #tag-input
│ │ ┌────────────────────────────┐  │ │
│ │ │ autocomplete-list          │  │ │  ← #tag-autocomplete (输入时显示)
│ │ │ matching tag 1             │  │ │
│ │ │ matching tag 2             │  │ │
│ │ └────────────────────────────┘  │ │
│ │                                  │ │
│ │ ── 可选标签 (点击添加) ─────── │ │
│ │ [# tagA] [# tagB] [# tagC]      │ │  ← #available-tags
│ │                                  │ │
│ └──────────────────────────────────┘ │
│                                      │
│ ┌──────────────────────────────────┐ │
│ │ #detail-empty (默认显示)         │ │
│ │ 👈 选择一个文件查看详情          │ │
│ └──────────────────────────────────┘ │
└──────────────────────────────────────┘
```

### 5.3 文件名区域

```html
<h3 id="detail-filename">{icon} {filename}</h3>
<p id="detail-path" class="detail-path">{fullPath}</p>
```

- 文件名前带类型图标
- 路径灰色小字显示

### 5.4 操作按钮区 `.detail-actions`

```html
<div class="detail-actions">
    <button id="btn-open-file">打开文件</button>
    <button id="btn-open-dir">打开目录</button>
    <button id="btn-remove-doc" class="btn-danger">移除文件</button>
</div>
```

**尺寸**：每个按钮 width: 80px

**功能**：

| 按钮 | 行为 |
|------|------|
| 打开文件 | `go.main.App.OpenFile(docId)` — 用系统默认程序打开 |
| 打开目录 | `go.main.App.OpenFileLocation(docId)` — 打开文件所在目录并选中 |
| 移除文件 | confirm 弹窗 → `RemoveDocument(docId)` → 刷新列表/标签/计数 → 隐藏详情 |

### 5.5 当前标签区 `#detail-tags`

```html
<div class="detail-section">
    <h4>标签</h4>
    <div id="detail-tags">
        <span class="detail-tag">
            tag1<span class="remove-tag" data-tag-id="1" title="移除">×</span>
        </span>
        <span class="detail-tag">
            tag2<span class="remove-tag" data-tag-id="2" title="移除">×</span>
        </span>
    </div>
</div>
```

**标签项样式**：`.detail-tag` — 蓝色背景圆角标签，右侧带 `×` 移除按钮

**移除标签交互**：
```
点击 .remove-tag
  │
  ├─ RemoveTagFromDocument(docId, tagId)
  ├─ 清空 tagCache[docId]
  ├─ selectDocument(docId) 刷新详情
  └─ refreshTags() 刷新侧栏标签
```

**无标签时**：显示灰色文字 "暂无标签"

### 5.6 标签输入区

```html
<div class="tag-input-row">
    <input type="text" id="tag-input" placeholder="输入新标签..." />
</div>
<div id="tag-autocomplete" class="autocomplete-list" style="display:none"></div>
```

**交互流程**：

```
输入标签名
  │
  ├─ input 事件 → handleTagAutocomplete()
  │   ├─ 从 allTags 中模糊匹配（最多 8 个）
  │   ├─ 渲染到 #tag-autocomplete
  │   └─ 无匹配 → 隐藏自动补全
  │
  ├─ 点击自动补全项 → 填入输入框 → 调用 handleAddTag()
  │
  └─ 按回车 / 点击 + 按钮 → handleAddTag()
      │
      ├─ 前置：tagName 不为空 + selectedDocId 存在
      ├─ AddTagToDocument(docId, tagName)
      ├─ 清空输入框 + 隐藏自动补全
      ├─ 清空 tagCache[docId]
      ├─ selectDocument(docId) 刷新详情
      ├─ refreshTags() 刷新侧栏
      └─ refreshDocuments() 刷新文件列表标签显示
```

### 5.7 可选标签区 `#available-tags`

```html
<div class="detail-section">
    <h4>可选标签 <span>（点击添加）</span></h4>
    <div id="available-tags">
        <span class="available-tag" data-tag-name="tagA"># tagA</span>
        <span class="available-tag" data-tag-name="tagB"># tagB</span>
    </div>
</div>
```

**逻辑**：从 `allTags` 中过滤掉当前文件已有的标签，显示剩余可添加的标签

**样式**：`.available-tag` — 虚线边框，`# ` 前缀

**点击交互**：
```
点击 .available-tag
  │
  ├─ AddTagToDocument(docId, tagName)
  ├─ 清空 tagCache[docId]
  ├─ selectDocument(docId) 刷新详情（该标签从可选移到当前）
  ├─ refreshTags()
  └─ refreshDocuments()
```

**全部已添加时**：显示灰色文字 "所有标签已添加"

---

## 六、批量操作逻辑

### 6.1 批量添加标签

```
触发：点击 #btn-batch-tag 或 #batch-tag-input 按回车
  │
  ├─ 前置：input 不为空 + multiSelectedIds.size > 0
  ├─ BatchAddTag(docIds, tagName) → 成功数
  ├─ 清空 tagCache + input + 两个选中集合 + select-all
  ├─ refreshDocuments + refreshTags
  └─ showToast
```

### 6.2 批量移除文件

```
触发：点击 #btn-batch-remove-docs
  │
  ├─ 前置：multiSelectedIds.size > 0
  ├─ confirm("确定要从列表中移除 N 个文件吗？（不会删除真实文件）")
  ├─ RemoveDocuments(docIds) → 成功数
  ├─ 清空两个选中集合 + select-all
  ├─ refreshDocuments + refreshTags + refreshFileTypeCounts + updateDocCount
  └─ showToast
```

### 6.3 批量移除标签

```
触发：点击 #btn-batch-remove-tags
  │
  ├─ 前置：multiSelectedIds.size > 0 + input 不为空 + 标签存在
  ├─ confirm("确定要从 N 个文件中移除标签「XXX」吗？")
  ├─ BatchRemoveTagFromDocuments(docIds, tagId) → 成功数
  ├─ 清空 tagCache + input + 两个选中集合 + select-all
  ├─ refreshDocuments + refreshTags
  ├─ 如有 selectedDocId → selectDocument 刷新详情
  └─ showToast
```

---

## 七、renderFileList 渲染流程

```
renderFileList() 调用时机：
  ├─ selectDocument() — active 高亮切换
  ├─ handleSelectAll / handleDeselectAll — 复选框刷新
  ├─ refreshDocuments() — 数据更新
  └─ 批量操作成功后（间接）

流程：
  1. documents 为空 → 显示 empty-state，隐藏 header
  2. updateBatchActions()
  3. 遍历 documents 生成 HTML：
     ├─ isActive = (doc.id === selectedDocId)
     ├─ isSelected = (multiSelectedIds.has(doc.id))
     ├─ isChecked = (selectedDocIds.has(doc.id))
     └─ 插入 tagCache 中的标签
  4. 设置 innerHTML
  5. 为每个 .file-item 绑定事件：
     ├─ .click-toggle-zone click → 切换复选框 + 更新状态 + 查看详情
     ├─ .file-info click → 仅查看详情
     └─ checkbox change → 更新状态
  6. 异步加载缺失的标签缓存
```

---

## 八、后端 API 对照表

| 前端操作 | 后端方法 | 参数 | 返回 |
|----------|----------|------|------|
| 批量添加标签 | `BatchAddTag` | `docIds []int64, tagName string` | `int` |
| 批量移除文件 | `RemoveDocuments` | `docIds []int64` | `int` |
| 批量移除标签 | `BatchRemoveTagFromDocuments` | `docIds []int64, tagID int64` | `int` |
| 单个移除文件 | `RemoveDocument` | `docID int64` | `error` |
| 给文件添加标签 | `AddTagToDocument` | `docID int64, tagName string` | `error` |
| 移除文件标签 | `RemoveTagFromDocument` | `docID int64, tagID int64` | `error` |
| 获取文件详情 | `GetDocument` | `docID int64` | `DocumentDetail` |
| 获取文件列表 | `ListDocuments` | `tagIDs []int64, search string, untagged bool, fileTypes []string` | `[]Document` |
| 获取所有标签 | `ListTags` | 无 | `[]TagWithCount` |
| 获取文件类型数量 | `GetFileTypeCounts` | 无 | `map[string]int` |
| 获取文档总数 | `GetDocumentCount` | 无 | `int` |
| 获取无标签文档数 | `GetUntaggedCount` | `fileTypes []string` | `int` |
| 打开文件 | `OpenFile` | `docID int64` | `error` |
| 打开目录 | `OpenFileLocation` | `docID int64` | `error` |
| 删除标签 | `DeleteTag` | `tagID int64` | `error` |
| 重命名标签 | `RenameTag` | `tagID int64, newName string` | `error` |

---

## 九、Rust 改写注意事项

1. **两个选中集合必须同步**：封装 `toggle_selection(docId)` / `clear_selection()` / `select_all()` 方法，避免遗漏。

2. **三个状态独立**：`selectedDocId`（详情联动）、`selectedDocIds`（复选框）、`multiSelectedIds`（批量操作）是独立的，不要合并。

3. **点击区域划分**：`.click-toggle-zone` 和 `.file-info` 的事件传播要严格隔离。

4. **标签输入框复用**：`#batch-tag-input` 同时服务「添加标签」和「移除标签」。

5. **批量操作后刷新链**：清空选中 → 刷新列表 → 刷新标签 → 刷新计数 → 刷新详情。

6. **active vs selected 可叠加**：CSS 优先级 active > selected > hover。
