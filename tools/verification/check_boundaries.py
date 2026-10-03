import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
tree = subprocess.check_output(['cargo', 'tree', '--locked', '--offline', '-p', 'astrology-engine',
                                '--target', 'wasm32-unknown-unknown', '--edges', 'normal', '--prefix', 'none'], cwd=ROOT, text=True)
names = {line.split()[0] for line in tree.splitlines()}
assert not names.intersection({'anise', 'worker', 'worker-sys', 'reqwest', 'tokio'}), tree
for path in (ROOT / 'src').rglob('*.rs'):
    text = path.read_text()
    assert not any(token in text for token in ['std::fs', 'std::net', 'std::env', 'include_bytes!', 'env!(']), path
manifest = json.loads((ROOT / 'tests/fixtures/manifest.json').read_text())
assert manifest['runtime'] == 'wasm32-unknown-unknown'
assert manifest['source_fixture_imports'] is False
print('Runtime dependency and ambient-I/O boundary checks passed')
