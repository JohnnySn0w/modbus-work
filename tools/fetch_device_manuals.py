"""Refresh the publicly available manuals listed in the provenance manifest."""
import json
import urllib.request
from pathlib import Path
root = Path(__file__).resolve().parents[1]
for item in json.loads((root / 'docs/reference/manual-sources.json').read_text(encoding='utf-8')):
    request = urllib.request.Request(item['source'], headers={'User-Agent': 'Mozilla/5.0'})
    data = urllib.request.urlopen(request, timeout=30).read()
    if not data.startswith(b'%PDF'):
        raise ValueError(f"Not a PDF: {item['source']}")
    (root / 'docs/reference' / item['file']).write_bytes(data)
    print(item['file'])
