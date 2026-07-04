import sys
with open('frontend/src/main.js', 'r', encoding='utf-8') as f:
    c = f.read()

# Find handleDeleteTag and everything until the next function
start = c.find('async function handleDeleteTag(tagId)')
if start < 0:
    print('NOT FOUND')
    sys.exit(1)

# Find the next function after handleDeleteTag
# Look for the pattern that starts the next function
rest = c[start:]
# Find the first line starting with 'async function' or 'function ' after the function ends
next_func = rest.find('\n// ', rest.find('\n', 200))
if next_func < 0:
    # Try finding the next async function  
    next_func = rest.find('\nasync function ', 200)
if next_func < 0:
    next_func = rest.find('\nfunction ', 200)

if next_func < 0:
    print('COULD NOT FIND NEXT FUNCTION')
    sys.exit(1)

# Extract current function
current_func = rest[:next_func].rstrip()
print('CURRENT FUNCTION LENGTH:', len(current_func))

new_func = """async function handleDeleteTag(tagId) {
    const tag = state.allTags.find(t => t.id === tagId);
    if (!tag) return;
    const item = document.querySelector(`.tag-item[data-tag-id="${tagId}"]`);
    if (!item) return;
    const nameSpan = item.querySelector('.tag-name');
    const origHtml = nameSpan.innerHTML;
    nameSpan.innerHTML = '\u786e\u5b9a\u5220\u9664? <span style="color:#D13438;cursor:pointer;font-weight:bold" class="inline-confirm">\\u2713</span> <span style="color:#27AE60;cursor:pointer;font-weight:bold" class="inline-cancel">\\u2715</span>';
    item.querySelector('.inline-confirm').addEventListener('click', async () => {
        try { await go.main.App.DeleteTag(tagId); } catch (e) {}
        await refreshTags(); state.tagCache = {}; await refreshDocuments();
        if (state.selectedDocId) await selectDocument(state.selectedDocId);
    });
    item.querySelector('.inline-cancel').addEventListener('click', () => { nameSpan.innerHTML = origHtml; });
}"""

c = c.replace(current_func, new_func)
print('REPLACED:', c.count('确定删除?'))

with open('frontend/src/main.js', 'w', encoding='utf-8') as f:
    f.write(c)
print('DONE')
