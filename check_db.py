import sqlite3, json

conn = sqlite3.connect('dist/data/data.db')

with open('dist/data/settings.json') as f:
    s = json.load(f)
folder = s.get('currentFolderPath', '')
print('currentFolderPath:', folder)
print()

rows = conn.execute('SELECT file_type, COUNT(*) FROM documents GROUP BY file_type').fetchall()
print('=== 文件类型数量（数据库原始） ===')
for r in rows:
    print(f'  {r[0]}: {r[1]}')
total = conn.execute('SELECT COUNT(*) FROM documents').fetchone()[0]
print(f'  总计: {total}')

if folder:
    prefix = folder.rstrip('/\\') + '/'
    print()
    print('=== 文件类型数量（排除选择路径下未打标签的文件） ===')
    rows = conn.execute("SELECT file_type, COUNT(*) FROM documents d WHERE NOT (d.path LIKE ? AND d.id NOT IN (SELECT document_id FROM document_tags)) GROUP BY file_type", [prefix + '%']).fetchall()
    for r in rows:
        print(f'  {r[0]}: {r[1]}')

print()
rows = conn.execute("SELECT t.id, t.name, t.color, COUNT(dt.document_id) as cnt FROM tags t LEFT JOIN document_tags dt ON t.id = dt.tag_id GROUP BY t.id ORDER BY cnt DESC, t.name").fetchall()
print('=== 标签数量 ===')
for r in rows:
    print(f'  #{r[1]}: {r[3]} 个文件')

# Also check how many untagged docs
untagged = conn.execute("SELECT COUNT(*) FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags)").fetchone()[0]
print(f'\n未分类文件数: {untagged}')

# List actual untagged docs
rows2 = conn.execute("SELECT id, filename, file_type FROM documents WHERE id NOT IN (SELECT document_id FROM document_tags) LIMIT 5").fetchall()
if rows2:
    print('未分类文件示例:')
    for r in rows2:
        print(f'  {r[0]}: {r[1]} ({r[2]})')
conn.close()
