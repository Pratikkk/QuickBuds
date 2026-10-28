#!/usr/bin/env python3
"""Stage the shared catalog and Android translations for the native macOS target."""
import json
from pathlib import Path
import shutil
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parent.parent
out = root / 'macOS/Sources/QuickBuds/Resources'
out.mkdir(parents=True, exist_ok=True)
shutil.copyfile(root / 'app/src/main/assets/models.json', out / 'models.json')
translations = {}
for path in sorted((root / 'app/src/main/res').glob('values*/strings.xml')):
    folder = path.parent.name
    if folder == 'values':
        locale = 'en'
    elif folder.startswith('values-'):
        locale = folder[7:].replace('-r', '-')
    else:
        continue
    translations[locale] = {
        entry.attrib['name']: ''.join(entry.itertext()).replace("\\'", "'").replace('\\"', '"').replace('\\n', '\n')
        for entry in ET.parse(path).getroot().findall('string')
    }
(out / 'strings.json').write_text(json.dumps(translations, ensure_ascii=False), encoding='utf-8')
