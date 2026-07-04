import sys
with open('frontend/src/main.js', 'r', encoding='utf-8') as f:
    c = f.read()

# 1. Remove group action buttons from HTML template
c = c.replace(
    '<span class="group-actions"><span class="group-action-btn" data-action="rename-group" title="重命名">\u270e</span><span class="group-action-btn" data-action="delete-group" title="删除分组">\u2716</span></span>',
    ''
)

# 2. Replace group header event handlers
old2_start = c.find('container.querySelectorAll(\'.tag-group-header\').forEach(header => {')
old2_end = c.find('    });', old2_start) + 6
old2 = c[old2_start:old2_end]

new2 = """container.querySelectorAll('.tag-group-header').forEach(header => {
        header.addEventListener('click', () => {
            const group = header.dataset.group;
            if (state.collapsedGroups.has(group)) {
                state.collapsedGroups.delete(group);
            } else {
                state.collapsedGroups.add(group);
            }
            renderTagList();
        });
        header.addEventListener('dblclick', (e) => {
            if (e.target.closest('.group-arrow') || e.target.closest('.group-count')) return;
            const group = header.dataset.group;
            if (group !== '__ungrouped__') startGroupRename(group, header);
        });
        header.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            e.stopPropagation();
            const group = header.dataset.group;
            if (group === '__ungrouped__') return;
            showGroupContextMenu(e.clientX, e.clientY, group);
        });
    });"""

c = c[:old2_start] + new2 + c[old2_end:]
print('replace 1 OK, len=', len(new2))

# 3. Add contextmenu to tag items
c = c.replace(
    """item.addEventListener('dblclick', (e) => { if (!e.target.closest('.tag-action-btn') && !e.target.closest('.tag-color-dot')) handleRenameTag(tagId); });""",
    """item.addEventListener('dblclick', (e) => { if (!e.target.closest('.tag-action-btn') && !e.target.closest('.tag-color-dot')) handleRenameTag(tagId); });
        item.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            showTagContextMenu(e.clientX, e.clientY, tagId);
        });"""
)

print('replace 2 OK')

# 4. Add showTagContextMenu and showGroupContextMenu before startGroupRename
c = c.replace(
    'function startGroupRename(group, headerEl) {',
    """function showTagContextMenu(x, y, tagId) {
    document.querySelectorAll('.tag-context-menu').forEach(el => el.remove());
    const menu = document.createElement('div');
    menu.className = 'tag-context-menu';
    menu.style.cssText = 'position:fixed;left:'+x+'px;top:'+y+'px;z-index:1000';
    menu.innerHTML = '<div class="context-menu-item" data-action="delete-tag">删除标签</div>';
    document.body.appendChild(menu);
    menu.querySelector('[data-action="delete-tag"]').addEventListener('click', () => {
        menu.remove();
        handleDeleteTag(tagId);
    });
    setTimeout(() => document.addEventListener('click', () => menu.remove(), { once: true }), 0);
}

function showGroupContextMenu(x, y, group) {
    document.querySelectorAll('.tag-context-menu').forEach(el => el.remove());
    const menu = document.createElement('div');
    menu.className = 'tag-context-menu';
    menu.style.cssText = 'position:fixed;left:'+x+'px;top:'+y+'px;z-index:1000';
    menu.innerHTML = '<div class="context-menu-item" data-action="delete-group">删除分组</div>';
    document.body.appendChild(menu);
    menu.querySelector('[data-action="delete-group"]').addEventListener('click', () => {
        menu.remove();
        const header = document.querySelector('.tag-group-header[data-group="'+group+'"]');
        if (!header) return;
        const nameSpan = header.querySelector('.group-name');
        const origText = nameSpan.textContent;
        nameSpan.innerHTML = '确认删除"'+group+'"? <span style="color:#D13438;cursor:pointer;font-weight:bold" class="inline-confirm">\\u2713</span> <span style="color:#27AE60;cursor:pointer;font-weight:bold" class="inline-cancel">\\u2715</span>';
        header.querySelector('.inline-confirm').addEventListener('click', async (e) => {
            e.stopPropagation();
            await Promise.all(state.allTags.filter(t => t.tag_group === group).map(t =>
                go.main.App.SetTagGroup(t.id, '')
            ));
            const idx = state.groups.indexOf(group);
            if (idx >= 0) state.groups.splice(idx, 1);
            refreshTags();
        });
        header.querySelector('.inline-cancel').addEventListener('click', (e) => {
            e.stopPropagation();
            nameSpan.innerHTML = origText;
        });
    });
    setTimeout(() => document.addEventListener('click', () => menu.remove(), { once: true }), 0);
}

function startGroupRename(group, headerEl) {"""
)

print('replace 3 OK')

with open('frontend/src/main.js', 'w', encoding='utf-8') as f:
    f.write(c)
print('ALL DONE')
