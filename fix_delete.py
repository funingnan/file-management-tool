import sys
with open('frontend/src/main.js', 'r', encoding='utf-8') as f:
    c = f.read()

old = """async function handleDeleteTag(tagId) {
    const tag = state.allTags.find(t => t.id === tagId);
    if (!tag) return;
    await go.main.App.DeleteTag(tagId);
    await refreshTags(); state.tagCache = {}; await refreshDocuments();
    if (state.selectedDocId) await selectDocument(state.selectedDocId);
}"""

new = """async function handleDeleteTag(tagId) {
    const tag = state.allTags.find(t => t.id === tagId);
    if (!tag) return;
    const item = document.querySelector(`.tag-item[data-tag-id="${tagId}"]`);
    if (!item) return;
    const nameSpan = item.querySelector('.tag-name');
    const origHtml = nameSpan.innerHTML;
    nameSpan.innerHTML = '\u786e\u5b9a\u5220\u9664? <span style="color:#D13438;cursor:pointer;font-weight:bold" class="inline-confirm">\u2713</span> <span style="color:#27AE60;cursor:pointer;font-weight:bold" class="inline-cancel">\u2715</span>';
    item.querySelector('.inline-confirm').addEventListener('click', async () => {
        try { await go.main.App.DeleteTag(tagId); } catch (e) {}
        await refreshTags(); state.tagCache = {}; await refreshDocuments();
        if (state.selectedDocId) await selectDocument(state.selectedDocId);
    });
    item.querySelector('.inline-cancel').addEventListener('click', () => { nameSpan.innerHTML = origHtml; });
}"""

c = c.replace(old, new)

with open('frontend/src/main.js', 'w', encoding='utf-8') as f:
    f.write(c)
print('OK')
