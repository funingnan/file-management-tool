// ========== 文件类型配置（模仿 Windows/微软配色） ==========
const FILE_TYPE_META = {
    pdf:   { label: 'PDF',   icon: 'PDF', iconSrc: 'src/icons/pdf.svg', bg: '#D13438', color: '#fff', name: 'PDF 文档' },
    docx:  { label: 'Word',  icon: 'DOC', iconSrc: 'src/icons/docx.svg', bg: '#2B579A', color: '#fff', name: 'Word 文档' },
    xlsx:  { label: 'Excel', icon: 'XLS', iconSrc: 'src/icons/xlsx.svg', bg: '#217346', color: '#fff', name: 'Excel 表格' },
    pptx:  { label: 'PPT',   icon: 'PPT', iconSrc: 'src/icons/pptx.svg', bg: '#C43E1C', color: '#fff', name: 'PPT 演示' },
    image: { label: '图片',   icon: 'IMG', iconSrc: 'src/icons/image.svg', bg: '#6B2FA0', color: '#fff', name: '图片' },
    video: { label: '视频',   icon: 'VID', iconSrc: 'src/icons/video.svg', bg: '#2D2D2D', color: '#fff', name: '视频' },
};

// ========== 全局状态 ==========
let state = {
    documents: [],
    tags: [],
    selectedDocId: null,
    selectedDocIds: new Set(),      // 复选框选中
    multiSelectedIds: new Set(),    // Shift/Ctrl 多选（独立于复选框）
    lastClickedIndex: -1,  // Shift 多选用
    currentFolderPath: '',  // 当前选择的文件夹路径
    activeTagIds: [],
    filterMode: 'all',
    fileTypeFilter: 'all',
    searchText: '',
    viewMode: 'list',
    graphMode: 'document',
    graphNetwork: null,
    allTags: [],
    settings: { enabledTypes: ['pdf','docx','xlsx','pptx','image','video'] },
    tagCache: {},  // docId → tags HTML 缓存
};

// ========== 初始化 ==========
document.addEventListener('DOMContentLoaded', async () => {
    // 先检查应用模式
    try {
        const modeInfo = await go.main.App.GetAppMode();
        if (modeInfo && modeInfo.mode === 'tag-picker') {
            initTagPicker(modeInfo.filePath);
            return;
        }
    } catch (err) { /* 正常应用模式 */ }

    await loadSettings();
    renderFileTypeFilter();
    await refreshTags();
    await refreshDocuments();
    await refreshFileTypeCounts();
    await updateDocCount();
    bindEvents();

    // 恢复上次选择的文件夹路径
    if (state.settings.currentFolderPath) {
        state.currentFolderPath = state.settings.currentFolderPath;
        state.filterMode = 'folder';
        document.getElementById('btn-select-folder').classList.add('has-path');
        const folderItem = document.querySelector('#file-type-filter .type-item[data-special="folder"]');
        if (folderItem) folderItem.classList.add('active');
        document.getElementById('btn-clear-filter').style.visibility = 'hidden';
        updatePathDisplay();
        await refreshDocuments();
        await refreshTags();
        await refreshFileTypeCounts();
        await updateDocCount();
    }
});

// ========== 标签选择器模式（右键菜单调用） ==========
let tpState = {
    filePath: '',
    allTags: [],       // 所有标签 [{id, name, count}]
    currentTags: [],   // 当前文件的标签 [{id, name}]
    searchText: '',
};

async function initTagPicker(filePath) {
    // 隐藏主界面，显示标签选择器
    document.getElementById('toolbar').style.display = 'none';
    document.getElementById('main-content').style.display = 'none';
    document.getElementById('graph-view').style.display = 'none';
    document.getElementById('modal-overlay').style.display = 'none';

    const pickerEl = document.getElementById('tag-picker-mode');
    pickerEl.style.display = 'flex';

    // 更新窗口标题
    const filename = filePath.split(/[/\\]/).pop();
    document.getElementById('tp-filename').textContent = filename;
    document.title = '添加标签 - ' + filename;

    tpState.filePath = filePath;

    // 加载标签数据
    await tpLoadTags();

    // 绑定事件
    tpBindEvents();

    // 聚焦搜索框
    document.getElementById('tp-search').focus();
}

async function tpLoadTags() {
    // 并行加载所有标签和当前文件标签
    const [allTags, currentTags] = await Promise.all([
        go.main.App.ListTags(),
        go.main.App.GetDocumentTagsByPath(tpState.filePath),
    ]);

    tpState.allTags = allTags || [];
    tpState.currentTags = currentTags || [];

    tpRenderCurrentTags();
    tpRenderTagList();
}

function tpBindEvents() {
    // 关闭按钮
    document.getElementById('tp-btn-close').addEventListener('click', () => {
        go.main.App.CloseTagPicker();
    });

    // 搜索输入
    const searchInput = document.getElementById('tp-search');
    searchInput.addEventListener('input', (e) => {
        tpState.searchText = e.target.value.trim();
        tpRenderTagList();
    });

    // 回车创建新标签
    searchInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && tpState.searchText) {
            const exactMatch = tpState.allTags.some(
                t => t.name.toLowerCase() === tpState.searchText.toLowerCase()
            );
            if (!exactMatch) {
                tpCreateAndAddTag(tpState.searchText);
            } else {
                // 精确匹配已有标签，直接添加
                const matched = tpState.allTags.find(
                    t => t.name.toLowerCase() === tpState.searchText.toLowerCase()
                );
                if (matched) {
                    tpAddTag(matched.name);
                }
            }
        }
    });

    // 创建按钮
    document.getElementById('tp-create-btn').addEventListener('click', () => {
        if (tpState.searchText) {
            tpCreateAndAddTag(tpState.searchText);
        }
    });

    // ESC 关闭
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            go.main.App.CloseTagPicker();
        }
    });
}

function tpRenderCurrentTags() {
    const section = document.getElementById('tp-current-section');
    const container = document.getElementById('tp-current-tags');

    if (tpState.currentTags.length === 0) {
        section.style.display = 'none';
        return;
    }

    section.style.display = 'block';
    container.innerHTML = '';

    tpState.currentTags.forEach(tag => {
        const el = document.createElement('span');
        el.className = 'tp-current-tag';
        el.innerHTML = `
            <span>${escapeHtml(tag.name)}</span>
            <button class="tp-tag-remove" data-tag-id="${tag.id}" title="移除">×</button>
        `;
        el.querySelector('.tp-tag-remove').addEventListener('click', () => {
            tpRemoveTag(tag.id, tag.name);
        });
        container.appendChild(el);
    });
}

function tpRenderTagList() {
    const listEl = document.getElementById('tp-tag-list');
    const emptyEl = document.getElementById('tp-empty');
    const createBtn = document.getElementById('tp-create-btn');
    const createText = document.getElementById('tp-create-text');

    const search = tpState.searchText.toLowerCase();
    const currentTagIds = new Set(tpState.currentTags.map(t => t.id));

    // 过滤标签
    let filtered = tpState.allTags;
    if (search) {
        filtered = filtered.filter(t => t.name.toLowerCase().includes(search));
    }

    // 排序：匹配搜索的排前面，然后按使用次数降序
    if (search) {
        filtered.sort((a, b) => {
            const aStart = a.name.toLowerCase().startsWith(search) ? 0 : 1;
            const bStart = b.name.toLowerCase().startsWith(search) ? 0 : 1;
            if (aStart !== bStart) return aStart - bStart;
            return b.count - a.count;
        });
    }

    // 判断是否需要显示"创建"按钮
    const exactMatch = tpState.allTags.some(t => t.name.toLowerCase() === search);
    if (search && !exactMatch) {
        createBtn.style.display = 'flex';
        createText.textContent = `创建并添加 "${tpState.searchText}"`;
    } else {
        createBtn.style.display = 'none';
    }

    // 渲染列表
    listEl.innerHTML = '';
    if (filtered.length === 0 && !search) {
        emptyEl.style.display = 'block';
        emptyEl.textContent = '暂无标签，请输入新标签名称';
        return;
    }
    emptyEl.style.display = filtered.length === 0 && search ? 'block' : 'none';
    emptyEl.textContent = '没有匹配的标签';

    filtered.forEach(tag => {
        const el = document.createElement('div');
        const isAdded = currentTagIds.has(tag.id);
        el.className = 'tp-tag-item' + (isAdded ? ' tp-tag-disabled' : '');

        const nameHtml = search ? highlightMatch(tag.name, search) : escapeHtml(tag.name);
        el.innerHTML = `
            <span class="tp-tag-name">${nameHtml}</span>
            <span class="tp-tag-count">${tag.count}</span>
        `;

        if (!isAdded) {
            el.addEventListener('click', () => tpAddTag(tag.name));
        }

        listEl.appendChild(el);
    });
}

async function tpAddTag(tagName) {
    try {
        await go.main.App.AddTagToFilePath(tpState.filePath, tagName);
        tpShowToast(`✓ 已添加标签 "${tagName}"`);
        // 重新加载
        await tpLoadTags();
        // 清空搜索
        document.getElementById('tp-search').value = '';
        tpState.searchText = '';
        tpRenderTagList();
    } catch (err) {
        tpShowToast('添加失败: ' + err, true);
    }
}

async function tpCreateAndAddTag(tagName) {
    await tpAddTag(tagName);
}

async function tpRemoveTag(tagID, tagName) {
    try {
        await go.main.App.RemoveTagFromFilePath(tpState.filePath, tagID);
        tpShowToast(`✓ 已移除标签 "${tagName}"`);
        await tpLoadTags();
    } catch (err) {
        tpShowToast('移除失败: ' + err, true);
    }
}

function tpShowToast(msg, isError) {
    const toast = document.getElementById('tp-toast');
    toast.textContent = msg;
    toast.style.background = isError ? '#D13438' : '#333';
    toast.classList.add('tp-toast-show');
    toast.style.display = 'block';

    clearTimeout(tpShowToast._timer);
    tpShowToast._timer = setTimeout(() => {
        toast.classList.remove('tp-toast-show');
        setTimeout(() => { toast.style.display = 'none'; }, 300);
    }, 1800);
}

function highlightMatch(text, search) {
    const lower = text.toLowerCase();
    const idx = lower.indexOf(search.toLowerCase());
    if (idx === -1) return escapeHtml(text);
    const before = text.slice(0, idx);
    const match = text.slice(idx, idx + search.length);
    const after = text.slice(idx + search.length);
    return escapeHtml(before) + '<span class="tp-highlight">' + escapeHtml(match) + '</span>' + escapeHtml(after);
}

// 模糊高亮：高亮搜索文本中每个按顺序出现的字符，支持多次出现
// 支持部分匹配：只要字符出现就高亮
function fuzzyHighlight(text, search) {
    if (!search) return escapeHtml(text);
    
    const searchLower = search.toLowerCase();
    const textLower = text.toLowerCase();
    
    // 标记需要高亮的位置
    const highlightPositions = new Set();
    
    // 1. 查找所有连续匹配的位置
    let startPos = 0;
    while (startPos < textLower.length) {
        const idx = textLower.indexOf(searchLower, startPos);
        if (idx === -1) break;
        for (let i = idx; i < idx + search.length; i++) {
            highlightPositions.add(i);
        }
        startPos = idx + 1;
    }
    
    // 2. 如果没有连续匹配，使用模糊匹配（按顺序出现的字符）
    if (highlightPositions.size === 0) {
        let searchIdx = 0;
        for (let i = 0; i < textLower.length && searchIdx < searchLower.length; i++) {
            if (textLower[i] === searchLower[searchIdx]) {
                highlightPositions.add(i);
                searchIdx++;
            }
        }
    }
    
    // 3. 如果仍然没有完全匹配，高亮所有出现的字符（不要求顺序）
    if (highlightPositions.size === 0) {
        for (let i = 0; i < textLower.length; i++) {
            if (searchLower.includes(textLower[i])) {
                highlightPositions.add(i);
            }
        }
    }
    
    // 4. 生成高亮 HTML
    let result = '';
    for (let i = 0; i < text.length; i++) {
        if (highlightPositions.has(i)) {
            result += '<span class="search-highlight">' + escapeHtml(text[i]) + '</span>';
        } else {
            result += escapeHtml(text[i]);
        }
    }
    
    return result;
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

// ========== 设置 ==========
async function loadSettings() {
    try {
        const s = await go.main.App.GetSettings();
        if (s && s.enabledTypes) state.settings = s;
    } catch (err) { /* use defaults */ }
}

async function saveSettings() {
    try {
        await go.main.App.SaveSettings(state.settings);
    } catch (err) {
        showToast('保存设置失败: ' + err, 'error');
    }
}

// 版本历史数据
const VERSION_HISTORY = [
    {
        version: 'v0.1.10',
        date: '2026-06-22',
        changes: [
            '扫描和选择路径按钮操作时保持固定不变，不再显示"扫描中..."',
            '选择路径后提示文字简化为"已完成加载"',
            '选择路径后图标不再被覆盖为旧emoji',
        ]
    },
    {
        version: 'v0.1.9',
        date: '2026-06-22',
        changes: [
            'UI 图标全面替换为 SVG（文件类型、删除、重命名、打开文件/目录、扫描、选择路径、列表视图）',
            '搜索算法重写：支持模糊匹配和部分字符匹配，按匹配度排序',
            '搜索关键字高亮显示（粉色背景）',
            '窗口最小宽度调整为 1180px',
            '设置页面文件类型勾选自动保存，取消保存按钮',
            '按钮样式优化（扫描文件、选择路径居中显示）',
            '右侧面板使用绝对定位，解决窗口拖拽时宽度抖动问题',
        ]
    },
    {
        version: 'v0.1.8',
        date: '2026-06-15',
        changes: [
            '批量标签选择器改为虚线边框+ # 前缀样式，与详情面板可选标签风格统一',
        ]
    },
    {
        version: 'v0.1.7',
        date: '2026-06-15',
        changes: [
            '设置面板文件类型行高降低，图标缩小至 26px',
        ]
    },
    {
        version: 'v0.1.6',
        date: '2026-06-15',
        changes: [
            '未分类图标问号改为红色，灰色底纹加深',
        ]
    },
    {
        version: 'v0.1.5',
        date: '2026-06-15',
        changes: [
            '未分类图标改为浅灰底色方块样式',
        ]
    },
    {
        version: 'v0.1.4',
        date: '2026-06-15',
        changes: [
            '修复：未分类数量未按设置中的文件类型过滤',
        ]
    },
    {
        version: 'v0.1.3',
        date: '2026-06-15',
        changes: [
            '修复：设置中取消某文件类型后，所有文件和未分类视图仍显示该类型的文件',
        ]
    },
    {
        version: 'v0.1.2',
        date: '2026-06-15',
        changes: [
            '设置齿轮图标改为 6 齿 SVG，大小调整为 20×20px',
        ]
    },
    {
        version: 'v0.1.1',
        date: '2026-06-15',
        changes: [
            '设置面板重构为左右分栏布局',
            '新增版本历史信息',
            '左下角显示版本号',
            '批量打标签支持矩阵式标签选择器',
            '设置按钮改为无边框文字链接样式',
            '复选框加大、全选对齐修复',
            '设置弹窗内复选框和图标加大',
        ]
    },
    {
        version: 'v0.1.0',
        date: '2026-06-14',
        changes: [
            '初始版本',
            '支持 PDF/Word/Excel/PPT/图片/视频 文件管理',
            '手动打标签、批量打标签',
            '标签筛选、文件类型筛选、搜索',
            'PDF 关联网络图谱、标签关联网络图谱',
            '智能匹配：文件改名/移动后自动保留标签',
            '设置面板：可选择启用的文件类型',
            '可选标签区域：点击直接添加已有标签',
        ]
    },
];

function showSettingsModal() {
    const allTypes = Object.keys(FILE_TYPE_META);
    const enabled = new Set(state.settings.enabledTypes);

    // 文件类型设置内容
    let fileTypesHtml = '<div class="settings-hint">勾选要管理的文件类型，取消勾选后扫描和侧栏将不显示该类型</div>';
    fileTypesHtml += '<div class="settings-filetypes-list">';
    allTypes.forEach(type => {
        const meta = FILE_TYPE_META[type];
        const checked = enabled.has(type) ? 'checked' : '';
        const iconHtml = meta.iconSrc 
            ? `<span class="file-type-icon" style="background:${meta.bg};flex-shrink:0;display:flex;align-items:center;justify-content:center"><img src="${meta.iconSrc}" style="width:18px;height:18px;filter:brightness(0) invert(1)" /></span>`
            : `<span class="file-type-icon" style="background:${meta.bg};color:${meta.color};flex-shrink:0">${meta.icon}</span>`;
        fileTypesHtml += `
            <label class="settings-filetype-item">
                <input type="checkbox" data-setting-type="${type}" ${checked} />
                ${iconHtml}
                <span class="settings-filetype-name">${meta.name}</span>
            </label>
        `;
    });
    fileTypesHtml += '</div>';

    // 版本信息内容
    let versionHtml = '<div class="settings-version-app">PDF 知识库 — 文件标签管理工具</div>';
    versionHtml += '<div class="settings-version-hint">版本更新历史</div>';
    VERSION_HISTORY.forEach(v => {
        versionHtml += `
            <div class="version-entry">
                <span class="version-tag">${v.version}</span>
                <span class="version-date">${v.date}</span>
                <ul class="version-changes">
                    ${v.changes.map(c => `<li>${escapeHtml(c)}</li>`).join('')}
                </ul>
            </div>
        `;
    });

    // 数据管理内容
    let dataHtml = '<div class="settings-hint">备份或恢复数据库文件。导入会替换当前所有数据，建议先导出备份。</div>';
    dataHtml += `
        <div class="settings-data-buttons">
            <button id="btn-export-db" class="settings-data-btn">
                <img src="src/icons/export.svg" />
                <div class="settings-data-btn-text">
                    <div class="settings-data-btn-title">导出数据库</div>
                    <div class="settings-data-btn-desc">将当前数据导出为 .db 文件</div>
                </div>
            </button>
            <button id="btn-import-db" class="settings-data-btn">
                <img src="src/icons/import.svg" />
                <div class="settings-data-btn-text">
                    <div class="settings-data-btn-title">导入数据库</div>
                    <div class="settings-data-btn-desc">从 .db 文件恢复数据（将替换当前数据）</div>
                </div>
            </button>
        </div>
    `;

    // 完整弹窗：左侧导航 + 右侧内容
    const html = `
        <div class="settings-sidebar">
            <div class="settings-sidebar-item active" data-tab="filetypes">文件类型</div>
            <div class="settings-sidebar-item" data-tab="data">数据管理</div>
            <div class="settings-sidebar-item" data-tab="version">版本信息</div>
        </div>
        <div class="settings-body">
            <div class="settings-section active" data-section="filetypes">${fileTypesHtml}</div>
            <div class="settings-section" data-section="data">${dataHtml}</div>
            <div class="settings-section" data-section="version">${versionHtml}</div>
        </div>
    `;

    showModal('设置', html);

    // 左侧导航切换
    document.querySelectorAll('.settings-sidebar-item').forEach(item => {
        item.addEventListener('click', () => {
            document.querySelectorAll('.settings-sidebar-item').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.settings-section').forEach(el => el.classList.remove('active'));
            item.classList.add('active');
            document.querySelector(`.settings-section[data-section="${item.dataset.tab}"]`).classList.add('active');
        });
    });

    // 文件类型勾选时自动保存
    document.querySelectorAll('[data-setting-type]').forEach(cb => {
        cb.addEventListener('change', async () => {
            const checkboxes = document.querySelectorAll('[data-setting-type]');
            const newEnabled = [];
            checkboxes.forEach(cb => {
                if (cb.checked) newEnabled.push(cb.dataset.settingType);
            });
            if (newEnabled.length === 0) {
                showToast('至少需要启用一种文件类型', 'error');
                cb.checked = true;
                return;
            }
            state.settings.enabledTypes = newEnabled;
            await saveSettings();
            renderFileTypeFilter();
            await refreshFileTypeCounts();
        });
    });

    // 数据导入导出
    document.getElementById('btn-export-db').addEventListener('click', async () => {
        try {
            const path = await go.main.App.ExportDatabase();
            if (path) showToast('已导出到: ' + path);
        } catch (err) { showToast('导出失败: ' + err, 'error'); }
    });
    document.getElementById('btn-import-db').addEventListener('click', async () => {
        try {
            const imported = await go.main.App.ImportDatabase();
            if (imported) {
                showToast('导入成功，正在刷新...');
                hideModal();
                await refreshDocuments();
                await refreshTags();
                await refreshFileTypeCounts();
                await updateDocCount();
            }
        } catch (err) { showToast('导入失败: ' + err, 'error'); }
    });

    // ESC键关闭设置窗口（不保存）
    const handleEsc = (e) => {
        if (e.key === 'Escape') {
            hideModal();
            document.getElementById('btn-settings').blur();
            document.removeEventListener('keydown', handleEsc);
        }
    };
    document.addEventListener('keydown', handleEsc);
}

// ========== 文件类型筛选栏（动态生成） ==========
function renderFileTypeFilter() {
    const container = document.getElementById('file-type-filter');
    const enabled = state.settings.enabledTypes;

    // "所有文件" 始终显示
    let html = `
        <div class="type-item active" data-type="all">
            <span class="tag-name"><span class="file-type-icon" style="background:#5B7EE5;color:#fff;width:20px;height:20px;font-size:8px">ALL</span> 所有文件</span>
            <span class="tag-count" id="all-count">—</span>
        </div>
    `;

    // 文件夹（固定显示，在所有文件下面）
    html += `
        <div class="type-item" data-special="folder">
            <span class="tag-name"><span class="file-type-icon" style="background:#F5A623;display:flex;align-items:center;justify-content:center;width:20px;height:20px"><img src="src/icons/folder.svg" style="width:14px;height:14px;filter:brightness(0) invert(1)" /></span> 文件夹</span>
            <span class="tag-count" id="folder-count">—</span>
        </div>
    `;

    // 按配置顺序渲染启用的类型
    const order = ['pdf','docx','xlsx','pptx','image','video'];
    order.forEach(type => {
        if (!enabled.includes(type)) return;
        const meta = FILE_TYPE_META[type];
        const iconHtml = meta.iconSrc
            ? `<span class="file-type-icon" style="background:${meta.bg};width:20px;height:20px;display:flex;align-items:center;justify-content:center"><img src="${meta.iconSrc}" style="width:14px;height:14px;filter:brightness(0) invert(1)" /></span>`
            : `<span class="file-type-icon" style="background:${meta.bg};color:${meta.color};width:20px;height:20px;font-size:${meta.icon.length > 1 ? '8px' : '10px'}">${meta.icon}</span>`;
        html += `
            <div class="type-item" data-type="${type}">
                <span class="tag-name">
                    ${iconHtml}
                    ${meta.label}
                </span>
                <span class="tag-count" id="type-${type}-count">0</span>
            </div>
        `;
    });

    // 未分类
    html += `
        <div class="type-item" data-special="untagged">
            <span class="tag-name"><span class="file-type-icon" style="background:#E8E8E8;color:#D13438;border:1px solid #ccc;width:20px;height:20px;font-size:10px">?</span> 未分类</span>
            <span class="tag-count" id="untagged-count">—</span>
        </div>
    `;

    container.innerHTML = html;

    // 绑定事件
    container.querySelectorAll('.type-item').forEach(item => {
        item.addEventListener('click', () => handleFileTypeFilter(item));
    });

    // 恢复当前选中状态
    const current = container.querySelector(`[data-type="${state.fileTypeFilter}"]`) ||
                    container.querySelector('[data-type="all"]');
    if (current) {
        container.querySelectorAll('.type-item').forEach(el => el.classList.remove('active'));
        current.classList.add('active');
    }
}

// ========== 事件绑定 ==========
function bindEvents() {
    document.getElementById('btn-scan').addEventListener('click', handleScan);
    document.getElementById('btn-select-folder').addEventListener('click', handleSelectFolder);
    document.getElementById('search-input').addEventListener('input', debounce(handleSearch, 300));
    document.getElementById('btn-view-list').addEventListener('click', () => switchView('list'));
    document.getElementById('btn-view-graph').addEventListener('click', () => switchView('graph'));
    document.getElementById('btn-graph-doc').addEventListener('click', () => loadGraph('document'));
    document.getElementById('btn-graph-tag').addEventListener('click', () => loadGraph('tag'));
    document.getElementById('graph-search').addEventListener('input', debounce(handleGraphSearch, 300));
    document.getElementById('select-all').addEventListener('change', handleSelectAll);
    document.getElementById('btn-deselect-all').addEventListener('click', handleDeselectAll);
    document.getElementById('btn-batch-tag').addEventListener('click', handleBatchTag);
    document.getElementById('btn-batch-remove-docs').addEventListener('click', handleBatchRemoveDocs);
    document.getElementById('btn-batch-remove-tags').addEventListener('click', handleBatchRemoveTags);
    document.getElementById('batch-tag-input').addEventListener('keydown', (e) => { if (e.key === 'Enter') handleBatchTag(); });
    document.getElementById('batch-tag-input').addEventListener('focus', () => showTagPicker('batch'));
    document.getElementById('batch-tag-input').addEventListener('input', () => filterTagPicker('batch'));
    document.getElementById('btn-clear-filter').addEventListener('click', clearTagFilter);
    document.getElementById('btn-open-file').addEventListener('click', handleOpenFile);
    document.getElementById('btn-open-dir').addEventListener('click', handleOpenDir);
    document.getElementById('btn-remove-doc').addEventListener('click', handleRemoveDoc);
    document.getElementById('tag-input').addEventListener('keydown', (e) => { if (e.key === 'Enter') handleAddTag(); });
    document.getElementById('tag-input').addEventListener('focus', () => showTagPicker('detail'));
    document.getElementById('tag-input').addEventListener('input', () => { handleTagAutocomplete(); filterTagPicker('detail'); });
    document.getElementById('tag-input').addEventListener('blur', () => { setTimeout(() => { hideAutocomplete(); hideTagPicker('detail'); }, 200); });
    document.getElementById('modal-overlay').addEventListener('click', (e) => { if (e.target === e.currentTarget) { hideModal(); document.getElementById('btn-settings').blur(); } });
    document.getElementById('btn-modal-close').addEventListener('click', () => { hideModal(); document.getElementById('btn-settings').blur(); });
    document.getElementById('btn-settings').addEventListener('click', showSettingsModal);
    // 点击空白关闭批量标签选择器
    document.addEventListener('click', (e) => {
        if (!e.target.closest('.tag-pick-wrapper') && !e.target.closest('#batch-tag-picker')) {
            hideTagPicker('batch');
        }
    });
}

// ========== 文件类型筛选 ==========
function handleFileTypeFilter(item) {
    const special = item.dataset.special;
    const type = item.dataset.type;

    document.querySelectorAll('#file-type-filter .type-item').forEach(el => el.classList.remove('active'));
    item.classList.add('active');

    if (special === 'untagged') {
        state.filterMode = 'untagged';
        state.fileTypeFilter = 'all';
    } else if (special === 'folder') {
        if (!state.currentFolderPath) {
            showToast('请先点击「选择路径」扫描文件');
            return;
        }
        state.filterMode = 'folder';
        state.fileTypeFilter = 'all';
        state.activeTagIds = [];
    } else {
        state.filterMode = 'all';
        state.fileTypeFilter = type || 'all';
        state.activeTagIds = [];
        updatePathDisplay();
    }

    document.getElementById('btn-clear-filter').style.visibility = 'hidden';
    renderTagList();
    refreshDocuments();
}

async function refreshFileTypeCounts() {
    try {
        const counts = await go.main.App.GetFileTypeCounts();
        let total = 0;
        state.settings.enabledTypes.forEach(ft => {
            const cnt = counts[ft] || 0;
            total += cnt;
            const el = document.getElementById(`type-${ft}-count`);
            if (el) el.textContent = cnt;
        });
        const allEl = document.getElementById('all-count');
        if (allEl) allEl.textContent = total;
    } catch (err) { /* ignore */ }
    try {
        const untagged = await go.main.App.GetUntaggedCount(state.settings.enabledTypes || []);
        const el = document.getElementById('untagged-count');
        if (el) el.textContent = untagged;
    } catch (err) { /* ignore */ }
    // 文件夹计数
    try {
        if (state.currentFolderPath) {
            const prefix = state.currentFolderPath.replace(/[\/\\]$/, '').toLowerCase();
            const allDocs = await go.main.App.ListDocuments([], '', false, state.settings.enabledTypes || []);
            const folderCount = allDocs.filter(d => {
                const docPath = d.path.toLowerCase();
                return docPath.startsWith(prefix + '\\') || docPath.startsWith(prefix + '/') || docPath === prefix;
            }).length;
            const el = document.getElementById('folder-count');
            if (el) el.textContent = folderCount;
        }
    } catch (err) { /* ignore */ }
}

// ========== 扫描 ==========
async function handleScan() {
    const folder = await go.main.App.SelectFolder();
    if (!folder) return;

    try {
        const result = await go.main.App.ScanFolder(folder, state.settings.enabledTypes);
        await refreshDocuments();
        await refreshTags();
        await refreshFileTypeCounts();
        await updateDocCount();

        let msg = `扫描完成：${result.total} 个文件`;
        if (result.relocated > 0) msg += `，智能匹配找回 ${result.relocated} 个`;
        showToast(msg);
    } catch (err) {
        showToast('扫描失败: ' + err, 'error');
    }
}

// ========== 选择路径（扫描+筛选） ==========
async function handleSelectFolder() {
    const folder = await go.main.App.SelectFolder();
    if (!folder) return;

    try {
        // 先扫描索引该文件夹
        await go.main.App.ScanFolder(folder, state.settings.enabledTypes);
        // 存储路径，切换到文件夹筛选
        state.currentFolderPath = folder;
        state.settings.currentFolderPath = folder;
        saveSettings();
        state.filterMode = 'folder';
        state.fileTypeFilter = 'all';
        state.activeTagIds = [];
        // 显示路径已选状态（圆点指示器）
        document.getElementById('btn-select-folder').classList.add('has-path');
        updatePathDisplay();
        // 刷新
        await refreshDocuments();
        await refreshTags();
        await refreshFileTypeCounts();
        await updateDocCount();
        // 高亮侧栏
        document.querySelectorAll('#file-type-filter .type-item').forEach(el => el.classList.remove('active'));
        const folderItem = document.querySelector('#file-type-filter .type-item[data-special="folder"]');
        if (folderItem) folderItem.classList.add('active');
        document.getElementById('btn-clear-filter').style.visibility = 'hidden';
        showToast('已完成加载');
    } catch (err) {
        showToast('操作失败: ' + err, 'error');
    }
}

// ========== 搜索 ==========
async function handleSearch() {
    state.searchText = document.getElementById('search-input').value.trim();
    await refreshDocuments();
}

// ========== 文档列表 ==========
async function refreshDocuments() {
    try {
        const untagged = state.filterMode === 'untagged';
        let fileTypes;
        if (state.fileTypeFilter === 'all') {
            fileTypes = state.settings.enabledTypes || [];
        } else {
            fileTypes = [state.fileTypeFilter];
        }
        state.documents = await go.main.App.ListDocuments(state.activeTagIds, state.searchText, untagged, fileTypes);
        // 文件夹模式：只显示该文件夹及其子目录下的文件
        if (state.filterMode === 'folder' && state.currentFolderPath) {
            const prefix = state.currentFolderPath.replace(/[\/\\]$/, '').toLowerCase();
            state.documents = state.documents.filter(d => {
                const docPath = d.path.toLowerCase();
                return docPath.startsWith(prefix + '\\') || docPath.startsWith(prefix + '/') || docPath === prefix;
            });
        }
        renderFileList();
    } catch (err) {
        console.error('刷新文档失败:', err);
    }
}

function fileIconHtml(type) {
    const meta = FILE_TYPE_META[type] || { icon: '?', bg: '#888', color: '#fff' };
    if (meta.iconSrc) {
        return `<span class="file-type-icon" style="background:${meta.bg};display:flex;align-items:center;justify-content:center"><img src="${meta.iconSrc}" style="width:18px;height:18px;filter:brightness(0) invert(1)" /></span>`;
    }
    return `<span class="file-type-icon" style="background:${meta.bg};color:${meta.color}">${meta.icon}</span>`;
}

function renderFileList() {
    const list = document.getElementById('file-list');
    const empty = document.getElementById('empty-state');
    const header = document.getElementById('file-list-header');

    if (!state.documents || state.documents.length === 0) {
        list.innerHTML = '';
        empty.style.display = 'flex';
        header.style.display = 'flex';
        updateBatchActions();
        return;
    }

    empty.style.display = 'none';
    header.style.display = 'flex';
    updateBatchActions();

    list.innerHTML = state.documents.map(doc => {
        const isActive = doc.id === state.selectedDocId;
        const isChecked = state.selectedDocIds.has(doc.id);
        const isSelected = state.multiSelectedIds.has(doc.id);
        const cachedTags = state.tagCache[doc.id] || '';
        return `
            <div class="file-item ${isActive ? 'active' : ''} ${isSelected ? 'selected' : ''} checkbox-mode" data-id="${doc.id}">
                <div class="click-toggle-zone">
                    <input type="checkbox" ${isChecked ? 'checked' : ''} />
                    ${fileIconHtml(doc.file_type)}
                </div>
                <div class="file-info">
                    <div class="file-name" title="${escapeHtml(doc.path)}">${state.searchText ? fuzzyHighlight(doc.filename, state.searchText) : escapeHtml(doc.filename)}</div>
                    <div class="file-tags" id="file-tags-${doc.id}">${cachedTags}</div>
                </div>
            </div>
        `;
    }).join('');

    list.querySelectorAll('.file-item').forEach(item => {
        const id = parseInt(item.dataset.id);
        const cb = item.querySelector('input[type="checkbox"]');
        const currentIndex = state.documents.findIndex(d => d.id === id);
        
        // Shift/Ctrl 多选（暂时禁用，保留代码）
        function handleMultiSelect(e) {
            /*
            if (e.shiftKey && state.lastClickedIndex >= 0) {
                const start = Math.min(state.lastClickedIndex, currentIndex);
                const end = Math.max(state.lastClickedIndex, currentIndex);
                for (let i = start; i <= end; i++) {
                    const docId = state.documents[i].id;
                    state.multiSelectedIds.add(docId);
                    const el = document.querySelector(`.file-item[data-id="${docId}"]`);
                    if (el) el.classList.add('selected');
                }
                state.lastClickedIndex = currentIndex;
                updateBatchActions();
                return true;
            }
            if (e.ctrlKey) {
                if (state.multiSelectedIds.has(id)) {
                    state.multiSelectedIds.delete(id);
                    item.classList.remove('selected');
                } else {
                    state.multiSelectedIds.add(id);
                    item.classList.add('selected');
                }
                state.lastClickedIndex = currentIndex;
                updateBatchActions();
                return true;
            }
            */
            state.lastClickedIndex = currentIndex;
            return false;
        }
        
        // 点击左侧区域（复选框+图标）→ 切换复选框 + 选中 + 查看详情
        item.querySelector('.click-toggle-zone').addEventListener('click', (e) => {
            if (handleMultiSelect(e)) { selectDocument(id); return; }
            if (e.target.type !== 'checkbox') {
                cb.checked = !cb.checked;
            }
            if (cb.checked) {
                state.selectedDocIds.add(id);
                state.multiSelectedIds.add(id);
            } else {
                state.selectedDocIds.delete(id);
                state.multiSelectedIds.delete(id);
            }
            updateBatchActions();
            selectDocument(id);
        });
        
        // 点击文件行其他区域 → 查看详情
        item.querySelector('.file-info').addEventListener('click', (e) => {
            handleMultiSelect(e);
            selectDocument(id);
        });
        
        cb.addEventListener('change', () => {
            if (cb.checked) {
                state.selectedDocIds.add(id);
                state.multiSelectedIds.add(id);
            } else {
                state.selectedDocIds.delete(id);
                state.multiSelectedIds.delete(id);
            }
            item.classList.toggle('selected', cb.checked);
            updateBatchActions();
        });
    });

    state.documents.forEach(doc => {
        if (!state.tagCache[doc.id]) loadFileTags(doc.id);
    });
}

async function loadFileTags(docId) {
    try {
        const detail = await go.main.App.GetDocument(docId);
        if (detail.tags && detail.tags.length > 0) {
            const html = detail.tags.map(t => `<span class="file-tag">${escapeHtml(t.name)}</span>`).join('');
            state.tagCache[docId] = html;
            const container = document.getElementById(`file-tags-${docId}`);
            if (container) container.innerHTML = html;
        }
    } catch (err) { /* ignore */ }
}

// ========== 标签面板 ==========
async function refreshTags() {
    try { state.allTags = await go.main.App.ListTags(); renderTagList(); }
    catch (err) { console.error('刷新标签失败:', err); }
}

function renderTagList() {
    const container = document.getElementById('tag-list');
    if (!state.allTags || state.allTags.length === 0) {
        container.innerHTML = '<div style="padding:12px 14px;color:#aaa;font-size:12px">暂无标签</div>';
        return;
    }
    container.innerHTML = state.allTags.map(tag => {
        const isActive = state.filterMode === 'tagged' && state.activeTagIds.includes(tag.id);
        return `
            <div class="tag-item ${isActive ? 'active' : ''}" data-tag-id="${tag.id}">
                <span class="tag-name"><img src="src/icons/tag.svg" style="width:13px;height:13px;vertical-align:middle;margin-right:2px" /># ${escapeHtml(tag.name)}</span>
                <span class="tag-count">${tag.count}</span>
                <div class="tag-actions">
                    <button class="tag-action-btn" data-action="rename" data-tip="重命名"><img src="src/icons/edit.svg" style="width:14px;height:14px" /></button>
                    <button class="tag-action-btn" data-action="delete" data-tip="删除"><img src="src/icons/delete.svg" style="width:14px;height:14px" /></button>
                </div>
            </div>
        `;
    }).join('');

    container.querySelectorAll('[data-tag-id]').forEach(item => {
        const tagId = parseInt(item.dataset.tagId);
        item.addEventListener('click', (e) => {
            if (e.target.closest('.tag-action-btn')) return;
            document.querySelectorAll('#file-type-filter .type-item').forEach(el => el.classList.remove('active'));
            state.fileTypeFilter = 'all';
            state.filterMode = 'tagged';
            toggleTagFilter(tagId);
        });
        item.querySelector('[data-action="rename"]').addEventListener('click', () => handleRenameTag(tagId));
        item.querySelector('[data-action="delete"]').addEventListener('click', () => handleDeleteTag(tagId));
    });
}

async function toggleTagFilter(tagId) {
    const idx = state.activeTagIds.indexOf(tagId);
    if (idx === -1) state.activeTagIds.push(tagId); else state.activeTagIds.splice(idx, 1);
    if (state.activeTagIds.length === 0) state.filterMode = 'all';
    document.getElementById('btn-clear-filter').style.visibility = state.filterMode !== 'all' ? 'visible' : 'hidden';
    renderTagList();
    await refreshDocuments();
    updateDocCount();
}

function clearTagFilter() {
    state.activeTagIds = [];
    state.filterMode = 'all';
    state.fileTypeFilter = 'all';
    document.getElementById('btn-clear-filter').style.visibility = 'hidden';
    document.querySelectorAll('#file-type-filter .type-item').forEach(el => el.classList.remove('active'));
    const allItem = document.querySelector('#file-type-filter .type-item[data-type="all"]');
    if (allItem) allItem.classList.add('active');
    renderTagList();
    updatePathDisplay();
    refreshDocuments();
}

// ========== 路径显示 ==========
function updatePathDisplay() {
    const el = document.getElementById('current-path');
    if (state.filterMode === 'folder' && state.currentFolderPath) {
        el.textContent = state.currentFolderPath;
        el.style.display = 'inline';
    } else {
        el.style.display = 'none';
    }
}

// ========== 文件详情 ==========
async function selectDocument(docId) {
    state.selectedDocId = docId;
    renderFileList();
    try { const doc = await go.main.App.GetDocument(docId); renderDetail(doc); }
    catch (err) { console.error('获取文档详情失败:', err); }
}

function renderDetail(doc) {
    document.getElementById('detail-empty').style.display = 'none';
    document.getElementById('detail-content').style.display = 'block';

    document.getElementById('detail-filename').innerHTML = `${fileIconHtml(doc.file_type)} ${escapeHtml(doc.filename)}`;
    document.getElementById('detail-path').textContent = doc.path;

    const tagContainer = document.getElementById('detail-tags');
    const currentTagIds = new Set();
    if (doc.tags && doc.tags.length > 0) {
        tagContainer.innerHTML = doc.tags.map(t => {
            currentTagIds.add(t.id);
            return `<span class="detail-tag">${escapeHtml(t.name)}<span class="remove-tag" data-tag-id="${t.id}" title="移除">×</span></span>`;
        }).join('');
        tagContainer.querySelectorAll('.remove-tag').forEach(el => {
            el.addEventListener('click', async () => {
                await go.main.App.RemoveTagFromDocument(doc.id, parseInt(el.dataset.tagId));
                delete state.tagCache[doc.id];
                await selectDocument(doc.id);
                await refreshTags();
            });
        });
    } else {
        tagContainer.innerHTML = '<span style="color:#aaa;font-size:12px">暂无标签</span>';
    }

    const availableContainer = document.getElementById('available-tags');
    const available = (state.allTags || []).filter(t => !currentTagIds.has(t.id));
    if (available.length > 0) {
        availableContainer.innerHTML = available.map(t =>
            `<span class="available-tag" data-tag-name="${escapeHtml(t.name)}" title="点击添加"># ${escapeHtml(t.name)}</span>`
        ).join('');
        availableContainer.querySelectorAll('.available-tag').forEach(el => {
            el.addEventListener('click', async () => {
                await go.main.App.AddTagToDocument(doc.id, el.dataset.tagName);
                delete state.tagCache[doc.id];
                await selectDocument(doc.id);
                await refreshTags();
                await refreshDocuments();
            });
        });
    } else {
        availableContainer.innerHTML = '<span style="color:#aaa;font-size:11px">所有标签已添加</span>';
    }
}

// ========== 移除文件 ==========
async function handleRemoveDoc() {
    if (!state.selectedDocId) return;
    if (!confirm('确定从列表中移除此文件？\n（不会删除真实文件）')) return;
    try {
        await go.main.App.RemoveDocument(state.selectedDocId);
        state.selectedDocId = null;
        document.getElementById('detail-empty').style.display = 'flex';
        document.getElementById('detail-content').style.display = 'none';
        await refreshDocuments();
        await refreshTags();
        await refreshFileTypeCounts();
        await updateDocCount();
        showToast('已移除');
    } catch (err) { showToast('移除失败: ' + err, 'error'); }
}

// ========== 添加标签 ==========
async function handleAddTag() {
    const input = document.getElementById('tag-input');
    const tagName = input.value.trim();
    if (!tagName || !state.selectedDocId) return;
    try {
        await go.main.App.AddTagToDocument(state.selectedDocId, tagName);
        delete state.tagCache[state.selectedDocId];
        input.value = '';
        hideAutocomplete();
        await selectDocument(state.selectedDocId);
        await refreshTags();
        await refreshDocuments();
    } catch (err) { showToast('添加标签失败: ' + err, 'error'); }
}

function handleTagAutocomplete() {
    const input = document.getElementById('tag-input');
    const value = input.value.trim().toLowerCase();
    const ac = document.getElementById('tag-autocomplete');
    if (!value) { hideAutocomplete(); return; }
    const matches = state.allTags.filter(t => t.name.toLowerCase().includes(value)).slice(0, 8);
    if (matches.length === 0) { hideAutocomplete(); return; }
    ac.innerHTML = matches.map(t => `<div class="autocomplete-item" data-name="${escapeHtml(t.name)}">${escapeHtml(t.name)}</div>`).join('');
    ac.style.display = 'block';
    ac.querySelectorAll('.autocomplete-item').forEach(item => {
        item.addEventListener('mousedown', async () => { input.value = item.dataset.name; hideAutocomplete(); await handleAddTag(); });
    });
}

function hideAutocomplete() { document.getElementById('tag-autocomplete').style.display = 'none'; }

// ========== 矩阵标签选择器 ==========
function showTagPicker(context) {
    const pickerId = context === 'batch' ? 'batch-tag-picker' : null;
    if (!pickerId) return; // detail 用已有 autocomplete

    const picker = document.getElementById(pickerId);
    if (!picker || !state.allTags || state.allTags.length === 0) return;

    renderTagPickerItems(picker, '');
    picker.style.display = 'flex';
}

function hideTagPicker(context) {
    const pickerId = context === 'batch' ? 'batch-tag-picker' : null;
    if (!pickerId) return;
    const picker = document.getElementById(pickerId);
    if (picker) picker.style.display = 'none';
}

function filterTagPicker(context) {
    const pickerId = context === 'batch' ? 'batch-tag-picker' : null;
    const inputId = context === 'batch' ? 'batch-tag-input' : null;
    if (!pickerId || !inputId) return;

    const picker = document.getElementById(pickerId);
    const input = document.getElementById(inputId);
    if (!picker || !input) return;

    renderTagPickerItems(picker, input.value.trim().toLowerCase());
    picker.style.display = 'flex';
}

function renderTagPickerItems(picker, filter) {
    const tags = filter
        ? state.allTags.filter(t => t.name.toLowerCase().includes(filter))
        : state.allTags;

    if (tags.length === 0) {
        picker.innerHTML = '<span style="color:#aaa;font-size:12px;padding:4px">无匹配标签</span>';
        return;
    }

    picker.innerHTML = tags.map(t =>
        `<span class="tag-picker-item" data-tag-name="${escapeHtml(t.name)}"># ${escapeHtml(t.name)}</span>`
    ).join('');

    picker.querySelectorAll('.tag-picker-item').forEach(item => {
        item.addEventListener('mousedown', (e) => {
            e.preventDefault(); // 阻止 blur
            const inputId = 'batch-tag-input';
            const input = document.getElementById(inputId);
            if (input) {
                input.value = item.dataset.tagName;
                hideTagPicker('batch');
                input.focus();
            }
        });
    });
}

// ========== 批量打标签 ==========
function updateBatchActions() {
    const count = state.multiSelectedIds.size;
    const actions = document.getElementById('batch-actions');
    actions.style.visibility = 'visible';
    if (count > 0) {
        document.getElementById('selected-count').textContent = `${count} 个已选`;
    } else {
        document.getElementById('selected-count').textContent = '0 个已选';
    }
}

function handleSelectAll() {
    const checked = document.getElementById('select-all').checked;
    state.multiSelectedIds.clear();
    state.selectedDocIds.clear();
    if (checked) {
        state.documents.forEach(d => {
            state.multiSelectedIds.add(d.id);
            state.selectedDocIds.add(d.id);
        });
    }
    renderFileList();
}

function handleDeselectAll() {
    state.multiSelectedIds.clear();
    state.selectedDocIds.clear();
    document.getElementById('select-all').checked = false;
    renderFileList();
}

async function handleBatchTag() {
    const input = document.getElementById('batch-tag-input');
    const tagName = input.value.trim();
    if (!tagName || state.multiSelectedIds.size === 0) return;
    try {
        const count = await go.main.App.BatchAddTag(Array.from(state.multiSelectedIds), tagName);
        state.multiSelectedIds.forEach(id => delete state.tagCache[id]);
        input.value = '';
        state.multiSelectedIds.clear();
        document.getElementById('select-all').checked = false;
        await refreshDocuments();
        await refreshTags();
        showToast(`已给 ${count} 个文件添加标签「${tagName}」`);
    } catch (err) { showToast('批量打标签失败: ' + err, 'error'); }
}

// ========== 批量移除文件 ==========
async function handleBatchRemoveDocs() {
    const count = state.multiSelectedIds.size;
    if (count === 0) return;
    if (!confirm(`确定要从列表中移除 ${count} 个文件吗？（不会删除真实文件）`)) return;
    try {
        const removed = await go.main.App.RemoveDocuments(Array.from(state.multiSelectedIds));
        state.multiSelectedIds.clear();
        document.getElementById('select-all').checked = false;
        await refreshDocuments();
        await refreshTags();
        await refreshFileTypeCounts();
        await updateDocCount();
        showToast(`已移除 ${removed} 个文件`);
    } catch (err) { showToast('批量移除失败: ' + err, 'error'); }
}

// ========== 批量移除标签 ==========
async function handleBatchRemoveTags() {
    const count = state.multiSelectedIds.size;
    if (count === 0) return;
    const input = document.getElementById('batch-tag-input');
    const tagName = input.value.trim();
    if (!tagName) { showToast('请先输入或选择要移除的标签', 'error'); return; }
    const tag = state.allTags.find(t => t.name === tagName);
    if (!tag) { showToast(`标签「${tagName}」不存在`, 'error'); return; }
    if (!confirm(`确定要从 ${count} 个文件中移除标签「${tagName}」吗？`)) return;
    try {
        const removed = await go.main.App.BatchRemoveTagFromDocuments(Array.from(state.multiSelectedIds), tag.id);
        state.multiSelectedIds.forEach(id => delete state.tagCache[id]);
        input.value = '';
        state.multiSelectedIds.clear();
        document.getElementById('select-all').checked = false;
        await refreshDocuments();
        await refreshTags();
        if (state.selectedDocId) await selectDocument(state.selectedDocId);
        showToast(`已从 ${removed} 个文件中移除标签「${tagName}」`);
    } catch (err) { showToast('批量移除标签失败: ' + err, 'error'); }
}

// ========== 标签操作 ==========
async function handleDeleteTag(tagId) {
    const tag = state.allTags.find(t => t.id === tagId);
    if (!tag || !confirm(`确定要删除标签「${tag.name}」吗？`)) return;
    try {
        await go.main.App.DeleteTag(tagId);
        await refreshTags(); await refreshDocuments();
        if (state.selectedDocId) await selectDocument(state.selectedDocId);
        showToast(`已删除标签「${tag.name}」`);
    } catch (err) { showToast('删除标签失败: ' + err, 'error'); }
}

async function handleRenameTag(tagId) {
    const tag = state.allTags.find(t => t.id === tagId);
    if (!tag) return;
    const newName = prompt('重命名标签:', tag.name);
    if (!newName || newName.trim() === tag.name) return;
    try {
        await go.main.App.RenameTag(tagId, newName.trim());
        await refreshTags(); await refreshDocuments();
        if (state.selectedDocId) await selectDocument(state.selectedDocId);
    } catch (err) { showToast('重命名失败: ' + err, 'error'); }
}

// ========== 打开文件 ==========
async function handleOpenFile() {
    if (!state.selectedDocId) return;
    try { await go.main.App.OpenFile(state.selectedDocId); }
    catch (err) { showToast('打开文件失败: ' + err, 'error'); }
}

async function handleOpenDir() {
    if (!state.selectedDocId) return;
    try { await go.main.App.OpenFileLocation(state.selectedDocId); }
    catch (err) { showToast('打开目录失败: ' + err, 'error'); }
}

// ========== 视图切换 ==========
function switchView(mode) {
    state.viewMode = mode;
    document.getElementById('main-content').style.display = mode === 'list' ? 'flex' : 'none';
    document.getElementById('graph-view').style.display = mode === 'graph' ? 'flex' : 'none';
    document.getElementById('btn-view-list').classList.toggle('active', mode === 'list');
    document.getElementById('btn-view-graph').classList.toggle('active', mode === 'graph');
    if (mode === 'graph') loadGraph(state.graphMode);
}

// ========== 网络图谱 ==========
async function loadGraph(mode) {
    state.graphMode = mode;
    document.getElementById('btn-graph-doc').classList.toggle('active', mode === 'document');
    document.getElementById('btn-graph-tag').classList.toggle('active', mode === 'tag');
    try {
        const data = mode === 'document' ? await go.main.App.GetDocumentGraph() : await go.main.App.GetTagGraph();
        renderGraph(data, mode);
    } catch (err) { showToast('加载网络图失败: ' + err, 'error'); }
}

function renderGraph(data, mode) {
    const container = document.getElementById('graph-container');
    if (!data || !data.nodes || data.nodes.length === 0) {
        container.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:#aaa">暂无数据，请先给文件添加标签</div>';
        return;
    }
    const colors = ['#4a90d9','#5cb85c','#f0ad4e','#d9534f','#9b59b6','#1abc9c','#e67e22','#3498db','#2ecc71','#e74c3c','#8e44ad','#16a085'];
    const nodes = new vis.DataSet(data.nodes.map((n, i) => ({
        id: n.id, label: n.label,
        size: Math.max(15, Math.min(50, n.size * 10)),
        color: { background: colors[i % colors.length], border: colors[i % colors.length], highlight: { background: '#ffd700', border: '#ffa500' } },
        font: { size: 12, color: '#333' },
        title: `${n.label}\n关联数: ${n.size}`,
    })));
    const edges = new vis.DataSet(data.edges.map(e => ({
        from: e.from, to: e.to, value: e.weight,
        title: `共享: ${e.weight}`,
        color: { color: '#aaa', highlight: '#4a90d9' },
        smooth: { type: 'continuous' }
    })));
    if (state.graphNetwork) state.graphNetwork.destroy();
    state.graphNetwork = new vis.Network(container, { nodes, edges }, {
        nodes: { shape: 'dot', borderWidth: 2, shadow: true },
        edges: { smooth: { enabled: true, type: 'continuous' }, scaling: { min: 1, max: 6 } },
        physics: { barnesHut: { gravitationalConstant: -3000, centralGravity: 0.1, springLength: 150, springConstant: 0.02, damping: 0.09 }, stabilization: { iterations: 200 } },
        interaction: { hover: true, tooltipDelay: 0, zoomView: true, dragView: true },
    });
    state.graphNetwork.on('doubleClick', (params) => {
        if (params.nodes.length > 0 && mode === 'document') { selectDocument(params.nodes[0]); switchView('list'); }
    });
}

function handleGraphSearch() {
    const query = document.getElementById('graph-search').value.trim().toLowerCase();
    if (!state.graphNetwork) return;
    if (!query) { state.graphNetwork.unselectAll(); return; }
    const matchIds = [];
    state.graphNetwork.body.data.nodes.forEach(node => { if (node.label.toLowerCase().includes(query)) matchIds.push(node.id); });
    if (matchIds.length > 0) { state.graphNetwork.selectNodes(matchIds); state.graphNetwork.fit({ nodes: matchIds, animation: true }); }
}

// ========== 工具函数 ==========
async function updateDocCount() {
    try { document.getElementById('doc-count').textContent = `${await go.main.App.GetDocumentCount()} 个文件`; }
    catch (err) { /* ignore */ }
}

function escapeHtml(str) {
    if (!str) return '';
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#039;');
}

function debounce(fn, ms) { let t; return function (...a) { clearTimeout(t); t = setTimeout(() => fn.apply(this, a), ms); }; }

function showToast(msg, type = 'info') {
    const toast = document.createElement('div');
    toast.textContent = msg;
    toast.style.cssText = `position:fixed;bottom:20px;right:20px;z-index:999;padding:10px 18px;border-radius:6px;font-size:13px;color:white;box-shadow:0 2px 10px rgba(0,0,0,0.15);background:${type === 'error' ? '#d9534f' : '#4a90d9'};`;
    document.body.appendChild(toast);
    setTimeout(() => { toast.style.opacity = '0'; toast.style.transition = 'opacity 0.3s'; setTimeout(() => toast.remove(), 300); }, 2500);
}

function showModal(title, bodyHtml) {
    const header = document.getElementById('modal-header');
    const titleEl = document.getElementById('modal-title');
    if (title) {
        titleEl.textContent = title;
        header.style.display = 'flex';
    } else {
        header.style.display = 'none';
    }
    document.getElementById('modal-body').innerHTML = bodyHtml;
    document.getElementById('modal-overlay').style.display = 'flex';
}

function hideModal() { document.getElementById('modal-overlay').style.display = 'none'; }

// ========== 调试功能 ==========
// Ctrl+Shift+D 切换区域调试底纹
// 未来新增调试区域只需在 CSS .debug-zones 下加一行
document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key === 'D') {
        document.body.classList.toggle('debug-zones');
    }
});

// 禁止 Ctrl+滚轮缩放
document.addEventListener('wheel', (e) => {
    if (e.ctrlKey) e.preventDefault();
}, { passive: false });

// 禁止 Ctrl+加减号缩放
document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && (e.key === '+' || e.key === '-' || e.key === '=' || e.key === '0')) {
        e.preventDefault();
    }
});

// 禁止 Ctrl+滚轮缩放
document.addEventListener('wheel', (e) => {
    if (e.ctrlKey) e.preventDefault();
}, { passive: false });

// 禁止 Ctrl+加减号缩放
document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && (e.key === '+' || e.key === '-' || e.key === '=' || e.key === '0')) {
        e.preventDefault();
    }
});
