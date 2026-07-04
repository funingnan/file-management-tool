import sys
with open('frontend/src/main.js', 'r', encoding='utf-8') as f:
    c = f.read()

# Remove all group-count spans
c = c.replace('<span class="group-count">${groups[groupName].length}</span>', '')
c = c.replace('<span class="group-count">${ungrouped.length}</span>', '')
c = c.replace('<span class="group-count">0</span>', '')

with open('frontend/src/main.js', 'w', encoding='utf-8') as f:
    f.write(c)
print('OK')
