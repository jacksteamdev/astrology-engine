# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

import argparse
import gzip
import hashlib
import json
import math
import subprocess
import tempfile
from pathlib import Path

from compare import compare, load

ROOT = Path(__file__).resolve().parents[2]


OVERLAP_ERROR = {
    'status': 500,
    'error': 'These house cusps overlap at this time and latitude. Choose Equal or Whole Sign houses.',
}


def overlapping_cusps(cusps):
    if cusps is None:
        return False
    if len(cusps) != 12:
        raise ValueError('Unexpected frozen cusp count')
    widths = [(cusps[(i + 1) % 12] - cusps[i]) % 360 for i in range(12)]
    return any(not math.isfinite(width) or width <= 0 for width in widths) or abs(sum(widths) - 360) > 1e-8


def migration_expectations(expected, overlap_count):
    """Apply only the agreed cusp-validation change to immutable baseline rows."""
    overlap_ids = sorted(key for key, value in expected.items()
                         if overlapping_cusps(value.get('chart', {}).get('cusps')))
    if len(overlap_ids) != overlap_count:
        raise ValueError(f'Expected {overlap_count} frozen overlapping sets, found {len(overlap_ids)}')
    admitted = set(overlap_ids)
    return {key: OVERLAP_ERROR.copy() if key in admitted else value for key, value in expected.items()}, overlap_ids


def adapter_migration(cases, overlap_count):
    overlap_ids = sorted(case['id'] for case in cases
                         if overlapping_cusps(case['value'].get('body', {}).get('cusps')))
    if len(overlap_ids) != overlap_count:
        raise ValueError(f'Expected {overlap_count} frozen HTTP overlapping sets, found {len(overlap_ids)}')
    admitted = set(overlap_ids)
    return [{**case, 'value': {'status': 500, 'body': {'error': 'internal'}}}
            if case['id'] in admitted else case for case in cases], overlap_ids


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
        adapter_cases = [json.loads(line) for line in (staging / 'adapter.jsonl').read_text().splitlines()]
        migrated_adapter, adapter_overlap_ids = adapter_migration(adapter_cases, 1)
        (staging / 'adapter-migration.jsonl').write_text(''.join(json.dumps(case) + '\n' for case in migrated_adapter))
        for prefix in ['', 'events-']:
            candidate = staging / (prefix + 'candidate.jsonl')
            command = ['node', str(ROOT / 'examples/cloudflare-worker/verify.mjs'),
                       '--dataset', str(args.dataset.resolve()), '--inputs', str(staging / (prefix + 'inputs.json')),
                       '--results', str(candidate)]
            if not prefix:
                command += ['--adapter', str(staging / 'adapter-migration.jsonl')]
            subprocess.run(command, cwd=ROOT, check=True)
            expected, overlap_ids = migration_expectations(load(staging / (prefix + 'results.jsonl')), 0 if prefix else 247)
            results[prefix or 'matrix'] = {**compare(expected, load(candidate)), 'intentional_overlap_rejections': overlap_ids}
    report = {'passed': all(result['passed'] for result in results.values()),
              'runtime': 'wasm32-unknown-unknown/workerd', 'dataset_sha256': actual_sha,
              'source_adapter_cases': 40, 'intentional_http_overlap_rejections': adapter_overlap_ids, 'comparisons': results}
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
