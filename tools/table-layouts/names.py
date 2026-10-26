"""Extracts `Class -> [fields]` from the game executable's data sections.

The deserializer's error messages ("<Class>의 <field>를 읽어들이는데 실패했다",
"failed to read <field> of <Class>") name every field it reads. Only plain
data is scanned; no code is analysed. Usage: names.py <CrimsonDesert.exe> <out.json>
"""
import sys, re, json, collections

data = open(sys.argv[1], 'rb').read()
pattern = re.compile(rb'([A-Za-z][A-Za-z0-9_:]{1,80})\xec\x9d\x98 (_?[A-Za-z][A-Za-z0-9_]{0,80})([^\x00]{0,120})\x00')
messages = collections.defaultdict(list)
for m in pattern.finditer(data):
    messages[m.group(1).decode()].append((m.start(), m.group(2).decode(), m.group(3).decode('utf8', 'replace')))
json.dump(messages, open(sys.argv[2], 'w'))
print(len(messages), "classes")
