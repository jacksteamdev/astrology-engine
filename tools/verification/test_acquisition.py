import hashlib
import importlib.util
import json
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('regenerate', ROOT / 'tools/regenerate.py')
regenerate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(regenerate)
import horizons


def rejects(kind, action):
    try:
        action()
    except kind:
        return
    raise AssertionError(f'Expected {kind.__name__}')


def run():
    with tempfile.TemporaryDirectory() as directory:
        cache = Path(directory)
        get = regenerate.recorded_get(cache, True)
        rejects(FileNotFoundError, lambda: get(regenerate.DE440S))
        request = {'url': regenerate.DE440S, 'params': {}}
        key = hashlib.sha256(json.dumps(request, sort_keys=True).encode()).hexdigest()
        raw = cache / (key + '.raw')
        raw.write_bytes(b'DAF/SPK manufactured acquisition bytes')
        rejects(ValueError, lambda: get(regenerate.DE440S))
        (cache / (key + '.json')).write_text(json.dumps({'request': request, 'sha256': regenerate.sha(raw)}))
        assert get(regenerate.DE440S).status_code == 200
        raw.write_bytes(b'corrupted cache')
        rejects(ValueError, lambda: get(regenerate.DE440S))
        response = SimpleNamespace(content=b'$$SOE\npartial', raise_for_status=lambda: None)
        with patch.object(regenerate.requests, 'get', return_value=response):
            rejects(regenerate.requests.RequestException, lambda: regenerate.recorded_get(cache, False)('https://example.test', {'epoch': 0}))
        assert len(list(cache.glob('*.raw'))) == 1
    replies = iter([SimpleNamespace(status_code=503, text='unavailable'), SimpleNamespace(status_code=200, text='$$SOE\n2451545,unused,1,2,3,4,5,6\n$$EOE')])
    with patch.object(horizons.time, 'sleep'):
        result = horizons.fetch_states('synthetic', 2451545, 2451545, 1, get=lambda *a, **k: next(replies))
    assert result[0].tolist() == [2451545]
    short = lambda *a, **k: SimpleNamespace(status_code=200, text='$$SOE\n2451545,unused,1,2,3,4,5,6\n$$EOE')
    rejects(RuntimeError, lambda: horizons.fetch_states('synthetic', 2451545, 2451547, 1, get=short))
    for residual in [1.01, float('nan'), float('inf')]:
        rejects(SystemExit, lambda: regenerate.assert_fit_within_floor('manufactured', residual))
    regenerate.assert_fit_within_floor('manufactured', 1.0)
    print('Acquisition cache, retry, incomplete response, short grid, and fit rejection checks passed')


if __name__ == '__main__':
    run()
