# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

import argparse
import gzip
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

from compare import compare, load

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    fixtures = ROOT / 'tests/fixtures'
    manifest = json.loads((fixtures / 'manifest.json').read_text())
    actual_sha = hashlib.sha256(args.dataset.read_bytes()).hexdigest()
    if actual_sha != manifest['dataset_sha256']:
        raise ValueError('The production dataset does not match the frozen baseline')
    results = {}
    with tempfile.TemporaryDirectory() as directory:
        staging = Path(directory)
        for name, identity in manifest['files'].items():
            content = gzip.decompress((fixtures / (name + '.gz')).read_bytes())
            if hashlib.sha256(content).hexdigest() != identity['sha256']:
                raise ValueError('Frozen capture hash mismatch: ' + name)
            (staging / name).write_bytes(content)
        for prefix in ['', 'events-']:
            candidate = staging / (prefix + 'candidate.jsonl')
            command = ['node', str(ROOT / 'examples/cloudflare-worker/verify.mjs'),
                       '--dataset', str(args.dataset.resolve()), '--inputs', str(staging / (prefix + 'inputs.json')),
                       '--results', str(candidate)]
            if not prefix:
                command += ['--adapter', str(staging / 'adapter.jsonl')]
            subprocess.run(command, cwd=ROOT, check=True)
            results[prefix or 'matrix'] = compare(load(staging / (prefix + 'results.jsonl')), load(candidate))
    report = {'passed': all(result['passed'] for result in results.values()),
              'runtime': 'wasm32-unknown-unknown/workerd', 'dataset_sha256': actual_sha,
              'source_adapter_cases': 40, 'comparisons': results}
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
