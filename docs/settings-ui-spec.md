# 设置界面 UI 设计与功能说明文档

本文档详细描述了文件标签管理工具中设置界面的UI设计、功能逻辑和技术实现，用于指导后续的 Rust 改写工作。

## 1. 概述

设置界面是一个模态弹窗，采用**左侧导航 + 右侧内容**的经典分栏布局。通过点击左侧面板底部的"设置"按钮触发显示。

**主要功能**：
- 文件类型管理：选择要管理的文件类型（PDF、DOCX、XLSX、PPTX、图片、视频）
- 数据管理：数据库导入/导出功能
- 版本信息：显示应用版本历史和更新日志

## 2. UI 设计

### 2.1 整体布局

```
┌─────────────────────────────────────────────┐
│ 设置                              [关闭按钮] │
├──────────────┬──────────────────────────────┤
│              │                              │
│  文件类型    │   [右侧内容区域]              │
│              │                              │
│  数据管理    │   根据左侧选择显示            │
│              │   对应的功能内容              │
│  版本信息    │                              │
│              │                              │
└──────────────┴──────────────────────────────┘
```

**尺寸规格**：
- 弹窗宽度：600px（最小560px，最大640px）
- 弹窗高度：480px
- 左侧导航栏宽度：140px
- 右侧内容区域：自适应剩余空间

### 2.2 触发按钮

**位置**：左侧面板底部（`.sidebar-footer`）

**HTML 结构**：
```html
<button id="btn-settings" class="btn-settings-link">
    <span class="settings-icon">
        <svg><!-- 齿轮图标 --></svg>
    </span>
    设置
</button>
```

**样式**：
- 背景：无
- 边框：无
- 字体：13px，颜色 #666
- 图标：20×20px 齿轮 SVG，颜色 #666
- 悬停效果：颜色变深为 #333

### 2.3 模态弹窗结构

**HTML 骨架**：
```html
<div id="modal-overlay" style="display:none">
    <div id="modal">
        <div id="modal-header">
            <h3 id="modal-title">设置</h3>
            <button id="btn-modal-close" class="btn btn-icon">✕</button>
        </div>
        <div id="modal-body">
            <!-- 动态生成的设置内容 -->
        </div>
    </div>
</div>
```

**样式**：
- 遮罩层：全屏固定定位，半透明黑色背景 `rgba(0,0,0,0.3)`，z-index: 200
- 弹窗主体：白色背景，圆角 8px，阴影效果
- 头部：高度 44px，底部边框分隔

### 2.4 左侧导航栏

**HTML 结构**：
```html
<div class="settings-sidebar">
    <div class="settings-sidebar-item active" data-tab="filetypes">文件类型</div>
    <div class="settings-sidebar-item" data-tab="data">数据管理</div>
    <div class="settings-sidebar-item" data-tab="version">版本信息</div>
</div>
```

**样式**：
- 背景：#f5f5f5
- 宽度：140px 固定
- 右边框：1px solid var(--border)
- 内边距：16px 0
- 导航项：
  - 内边距：8px 16px
  - 字体：13px，颜色 #555
  - 左边框：3px 透明
  - 悬停效果：背景 #e8e8e8
  - 激活状态：
    - 背景：#ddeeff
    - 颜色：var(--primary)
    - 字体粗细：600
    - 左边框颜色：var(--primary)

### 2.5 右侧内容区域

**样式**：
- 弹性布局：flex: 1
- 内边距：20px
- 垂直滚动：overflow-y: auto
- 内容区域切换：默认 display: none，激活时 display: block

## 3. 功能逻辑

### 3.1 状态管理

**全局状态对象**：
```javascript
state: {
    settings: {
        enabledTypes: ['pdf', 'docx', 'xlsx', 'pptx', 'image', 'video']
    }
}
```

**相关函数**：
- `loadSettings()`：应用启动时从后端读取设置
- `saveSettings()`：将设置保存到后端

### 3.2 设置弹窗显示逻辑

**函数**：`showSettingsModal()`

**执行流程**：
1. 获取所有文件类型和当前启用状态
2. 动态生成三个 Tab 的 HTML 内容
3. 组合成完整的设置弹窗 HTML
4. 调用 `showModal('设置', html)` 显示弹窗
5. 绑定各种事件监听器

### 3.3 Tab 切换逻辑

**实现方式**：
```javascript
document.querySelectorAll('.settings-sidebar-item').forEach(item => {
    item.addEventListener('click', () => {
        // 移除所有 active 类
        document.querySelectorAll('.settings-sidebar-item').forEach(el => el.classList.remove('active'));
        document.querySelectorAll('.settings-section').forEach(el => el.classList.remove('active'));
        
        // 添加当前项的 active 类
        item.classList.add('active');
        document.querySelector(`.settings-section[data-section="${item.dataset.tab}"]`).classList.add('active');
    });
});
```

### 3.4 文件类型设置逻辑

**交互流程**：
1. 显示所有文件类型的复选框列表
2. 每个复选框包含：文件类型图标、类型名称
3. 勾选/取消勾选时自动保存

**数据验证**：
- 至少需要启用一种文件类型
- 尝试取消全部勾选时，显示错误提示并恢复勾选状态

**保存流程**：
```javascript
// 收集所有已勾选的类型
const newEnabled = [];
checkboxes.forEach(cb => {
    if (cb.checked) newEnabled.push(cb.dataset.settingType);
});

// 验证至少有一种类型
if (newEnabled.length === 0) {
    showToast('至少需要启用一种文件类型', 'error');
    cb.checked = true;
    return;
}

// 更新状态并保存
state.settings.enabledTypes = newEnabled;
await saveSettings();

// 刷新相关 UI
renderFileTypeFilter();
await refreshFileTypeCounts();
```

### 3.5 数据管理逻辑

**导出数据库**：
```javascript
document.getElementById('btn-export-db').addEventListener('click', async () => {
    try {
        const path = await go.main.App.ExportDatabase();
        if (path) showToast('已导出到: ' + path);
    } catch (err) { 
        showToast('导出失败: ' + err, 'error'); 
    }
});
```

**导入数据库**：
```javascript
document.getElementById('btn-import-db').addEventListener('click', async () => {
    try {
        const imported = await go.main.App.ImportDatabase();
        if (imported) {
            showToast('导入成功，正在刷新...');
            hideModal();
            
            // 刷新所有相关数据
            await refreshDocuments();
            await refreshTags();
            await refreshFileTypeCounts();
            await updateDocCount();
        }
    } catch (err) { 
        showToast('导入失败: ' + err, 'error'); 
    }
});
```

### 3.6 关闭弹窗逻辑

**触发方式**：
1. 点击关闭按钮（✕）
2. 点击遮罩层
3. 按下 ESC 键

**ESC 键处理**：
```javascript
const handleEsc = (e) => {
    if (e.key === 'Escape') {
        hideModal();
        document.getElementById('btn-settings').blur();
        document.removeEventListener('keydown', handleEsc);
    }
};
document.addEventListener('keydown', handleEsc);
```

## 4. 功能说明

### 4.1 文件类型设置

**功能描述**：
- 显示所有支持的文件类型（PDF、DOCX、XLSX、PPTX、图片、视频）
- 每种类型显示对应的图标和名称
- 用户可以勾选/取消勾选要管理的文件类型
- 设置立即生效，无需确认

**影响范围**：
- 侧栏文件类型筛选栏
- 文件列表显示
- 文件扫描功能
- 搜索功能

**默认配置**：
```go
var defaultSettings = Settings{
    EnabledTypes: []string{"pdf", "docx", "xlsx", "pptx", "image", "video"},
}
```

### 4.2 数据管理

**功能描述**：
- **导出数据库**：将当前数据库文件导出为 `.db` 文件
- **导入数据库**：从 `.db` 文件恢复数据，会替换当前所有数据

**注意事项**：
- 导入操作会替换当前所有数据
- 建议在导入前先导出备份
- 导入成功后会自动刷新所有数据

### 4.3 版本信息

**功能描述**：
- 显示应用名称："PDF 知识库 — 文件标签管理工具"
- 显示版本历史列表
- 每个版本包含：版本号、发布日期、更新内容

**版本历史格式**：
```html
<div class="version-entry">
    <span class="version-tag">v0.1.10</span>
    <span class="version-date">2024-01-15</span>
    <ul class="version-changes">
        <li>更新内容1</li>
        <li>更新内容2</li>
    </ul>
</div>
```

## 5. 技术实现细节

### 5.1 后端 API

**Wails 绑定**：
- `GetSettings() (*Settings, error)`：获取当前设置
- `SaveSettings(s *Settings) error`：保存设置

**数据模型**：
```go
type Settings struct {
    EnabledTypes []string `json:"enabledTypes"` // 启用的文件类型
}
```

**持久化**：
- 存储路径：可执行文件同目录下 `data/settings.json`
- 文件格式：JSON
- 读取失败时返回默认设置
- 写入时自动创建目录

### 5.2 前端状态管理

**状态初始化**：
```javascript
state: {
    settings: {
        enabledTypes: ['pdf', 'docx', 'xlsx', 'pptx', 'image', 'video']
    }
}
```

**状态同步**：
- 应用启动时：`loadSettings()` 从后端读取
- 设置变更时：`saveSettings()` 保存到后端
- UI 更新：设置变更后立即刷新相关组件

### 5.3 事件处理

**事件绑定时机**：
- 设置弹窗显示时绑定所有事件
- 弹窗关闭时移除 ESC 键监听器

**事件类型**：
- 导航切换：点击左侧导航项
- 文件类型设置：复选框 change 事件
- 数据导入导出：按钮 click 事件
- 弹窗关闭：ESC 键、遮罩层点击、关闭按钮点击

### 5.4 错误处理

**前端错误处理**：
- 文件类型设置：至少保留一种类型
- 数据导入导出：try-catch 捕获异常，显示错误提示

**后端错误处理**：
- 设置读取：文件不存在或解析失败时返回默认设置
- 设置写入：创建目录失败时返回错误

## 6. Rust 改写建议

### 6.1 数据模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled_types: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_types: vec![
                "pdf".to_string(),
                "docx".to_string(),
                "xlsx".to_string(),
                "pptx".to_string(),
                "image".to_string(),
                "video".to_string(),
            ],
        }
    }
}
```

### 6.2 状态管理

建议使用 Rust 的状态管理方案：
- 使用 `Arc<Mutex<Settings>>` 或类似机制
- 实现 `load_settings()` 和 `save_settings()` 函数
- 确保线程安全

### 6.3 UI 框架选择

根据项目需求选择合适的 Rust GUI 框架：
- **Tauri**：Web 技术栈，与现有前端兼容性好
- **Iced**：原生 Rust GUI，性能好
- **egui**：即时模式 GUI，简单易用

### 6.4 错误处理

使用 Rust 的错误处理模式：
```rust
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("At least one file type must be enabled")]
    NoFileTypeEnabled,
}
```

## 7. 样式参考

### 7.1 CSS 变量

```css
:root {
    --primary: #1890ff;  // 主色调
    --border: #e8e8e8;   // 边框颜色
}
```

### 7.2 关键样式类

- `.btn-settings-link`：设置按钮
- `.settings-sidebar`：左侧导航栏
- `.settings-sidebar-item`：导航项
- `.settings-body`：右侧内容区域
- `.settings-section`：内容区域
- `.version-entry`：版本信息条目

### 7.3 响应式考虑

当前设计为固定尺寸，Rust 改写时可考虑：
- 最小/最大尺寸限制
- 窗口缩放适应
- 高 DPI 屏幕支持

---

**文档版本**：v1.0  
**最后更新**：2024年  
**适用项目**：文件标签管理工具 Rust 改写