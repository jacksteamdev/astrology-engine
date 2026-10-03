import argparse
import hashlib
import json
import math
import os
import platform
import subprocess
import sys
import time
from collections import namedtuple
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).parent / 'jpl'))

import requests
from cheby_fit import fit_type2_segment
from fit_gate import assert_fit_within_floor
from horizons import fetch_states
from spk_writer import write_type2_spk
from validate import validate_spk

DE440S = 'https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp'
START = 2396393.5
STOP = 2506696.5
BODIES = {'ceres': (2000001, '2000001'), 'chiron': (2002060, '2060;')}
HttpResult = namedtuple('HttpResult', 'status_code text')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, data):
    temporary = path.with_suffix(path.suffix + '.partial')
    temporary.write_text(json.dumps(data, indent=2) + '\n')
    temporary.replace(path)


def recorded_get(cache, offline):
    def get(url, params=None, timeout=180):
        identity = {'url': url, 'params': params or {}}
        key = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
        data = cache / (key + '.raw')
        metadata = cache / (key + '.json')
        if data.exists() or metadata.exists():
            if not data.exists() or not metadata.exists():
                raise ValueError('Incomplete acquisition record: ' + key)
            record = json.loads(metadata.read_text())
            if record['sha256'] != sha(data) or record['request'] != identity:
                raise ValueError('Acquisition hash or request mismatch: ' + key)
            content = data.read_bytes()
        else:
            if offline:
                raise FileNotFoundError('Offline input missing: ' + key)
            response = requests.get(url, params=params, timeout=timeout)
            response.raise_for_status()
            content = response.content
            if not params and not content.startswith(b'DAF/SPK '):
                raise ValueError('DE440s response is not an SPK')
            if params and (b'$$SOE' not in content or b'$$EOE' not in content):
                raise requests.RequestException('Incomplete Horizons vector response')
            data.with_suffix('.partial').write_bytes(content)
            data.with_suffix('.partial').replace(data)
            record = {'request': identity, 'resolved_url': response.url, 'retrieved_at': datetime.now(timezone.utc).isoformat(),
                      'sha256': sha(data), 'bytes': len(content), 'headers': dict(response.headers)}
            if params:
                record['solution_header'] = content.decode().split('$$SOE', 1)[0]
            write_json(metadata, record)
        return HttpResult(200, content.decode() if params else '')
    return get


def acquire(cache):
    cache.mkdir(parents=True, exist_ok=True)
    get = recorded_get(cache, False)
    get(DE440S, timeout=300)
    for name, (_, command) in BODIES.items():
        print(f'Acquiring {name} fit vectors', flush=True)
        jd, _, _ = fetch_states(command, START, STOP, 2.0, get=get)
        records = int(math.floor((jd[-1] - jd[0]) / 32.0 + 1e-9))
        validation_stop = jd[0] + 32.0 * records - 0.5
        fetch_states(command, float(jd[0]), float(validation_stop), (validation_stop-jd[0])/3999, get=get)
    records = {p.name: sha(p) for p in sorted(cache.glob('*.raw'))}
    write_json(cache/'acquisition.json', {'complete': True, 'inputs': records, 'historical_production_reproduction': 'not established'})


def build(cache, out, builder):
    source = json.loads((cache/'acquisition.json').read_text())
    if not source['complete']:
        raise ValueError('Acquisition is incomplete')
    for name, expected in source['inputs'].items():
        if sha(cache/name) != expected:
            raise ValueError('Input hash mismatch: '+name)
    if out.exists():
        raise FileExistsError('Use a new output directory: '+str(out))
    out.mkdir(parents=True)
    get = recorded_get(cache, True)
    get(DE440S)
    key = hashlib.sha256(json.dumps({'url': DE440S, 'params': {}}, sort_keys=True).encode()).hexdigest()
    (out/'de440s.bsp').write_bytes((cache/(key+'.raw')).read_bytes())
    kernel_report = {}
    started = time.monotonic()
    for name, (target, command) in BODIES.items():
        print(f'Fitting {name} Type-2 SPK', flush=True)
        jd, pos, _ = fetch_states(command, START, STOP, 2.0, get=get)
        segment, fit = fit_type2_segment(jd, pos, target=target, center=10, frame=1, intlen_days=32.0, degree=12,
                                        segid=f'official Horizons {command} Type-2')
        path = out/(name+'.bsp')
        write_type2_spk(str(path), segment, internal_name=name+'-JPL-Horizons')
        start = 2451545.0 + segment.init_sec / 86400.0
        stop = 2451545.0 + (segment.init_sec + segment.intlen_sec * len(segment.records)) / 86400.0
        validation = validate_spk(str(path), target, command, start, stop-0.5, 4000, get=get)
        assert_fit_within_floor(name, validation['worst_ang_arcsec'])
        kernel_report[name] = {'fit': fit, 'validation': validation, 'sha256': sha(path)}
    write_json(out/'kernels.json', kernel_report)
    builder_sha256 = sha(builder)
    fingerprint = builder_sha256[:8]
    command = [str(builder), 'fit', '--de440s', str(out/'de440s.bsp'), '--ceres', str(out/'ceres.bsp'), '--chiron', str(out/'chiron.bsp'),
               '--out', str(out/'cheb.bin'), '--report', str(out/'residual-report.json'), '--manifest', str(out/'manifest.json'), '--fingerprint', fingerprint]
    subprocess.run(command, check=True)
    manifest = json.loads((out/'manifest.json').read_text())
    if sha(out/'cheb.bin') != manifest['sha256']:
        raise ValueError('Generated blob hash mismatch')
    subprocess.run(['node', 'examples/cloudflare-worker/verify.mjs', '--dataset', str(out/'cheb.bin'), '--report', str(out/'wasm-validation.json')], cwd=ROOT, check=True)
    write_json(out/'provenance.json', {'usable': True, 'inputs': source, 'outputs': {p.name:sha(p) for p in out.iterdir() if p.is_file()},
                                     'elapsed_seconds':time.monotonic()-started,'platform':platform.platform(),'python':sys.version,
                                     'command':command,'cargo_lock_sha256':sha(ROOT/'Cargo.lock'), 'builder_sha256':builder_sha256,
                                     'python_lock_sha256':sha(ROOT/'tools/jpl/requirements.lock'),
                                     'historical_production_reproduction':'not established'})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('stage', choices=['acquire','build'])
    parser.add_argument('--cache', type=Path, required=True)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--builder', type=Path, default=ROOT/'target/release/dataset-builder')
    args = parser.parse_args()
    if args.stage == 'acquire':
        acquire(args.cache.resolve())
    elif args.out:
        build(args.cache.resolve(), args.out.resolve(), args.builder.resolve())
    else:
        parser.error('build requires --out')


if __name__ == '__main__':
    main()
