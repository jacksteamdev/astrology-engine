# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

"""Negative controls for the narrow frozen-chart migration policy."""
import copy
import gzip
import json
from pathlib import Path

from compare import compare
from parity import OVERLAP_ERROR, adapter_migration, migration_expectations


def test_migration_policy():
    root = Path(__file__).resolve().parents[2]
    with gzip.open(root / 'tests/fixtures/results.jsonl.gz', 'rt') as stream:
        original = {row['id']: row['value'] for row in map(json.loads, stream)}
    before = copy.deepcopy(original)
    expected, admitted = migration_expectations(original, 247)
    assert original == before
    assert len(admitted) == 247
    assert all(expected[key] == OVERLAP_ERROR for key in admitted)
    assert all(expected[key] == value for key, value in original.items() if key not in admitted)
    assert compare(expected, copy.deepcopy(expected))['passed']
    altered = copy.deepcopy(expected)
    altered[admitted[0]] = original[admitted[0]]
    assert not compare(expected, altered)['passed']
    altered[admitted[0]] = {'status': 500, 'error': 'unrelated failure'}
    assert not compare(expected, altered)['passed']
    unaffected = next(key for key in original if key not in admitted)
    altered = copy.deepcopy(expected)
    altered[unaffected] = OVERLAP_ERROR
    assert not compare(expected, altered)['passed']
    assert not compare(expected, {key: value for key, value in expected.items() if key != admitted[0]})['passed']
    assert not compare(expected, {**expected, 'extra': OVERLAP_ERROR})['passed']
    try:
        migration_expectations(original, 246)
    except ValueError:
        pass
    else:
        raise AssertionError('Unexpected admission count was accepted')
    with gzip.open(root / 'tests/fixtures/adapter.jsonl.gz', 'rt') as stream:
        adapter = [json.loads(line) for line in stream]
    original_adapter = copy.deepcopy(adapter)
    migrated, admitted_http = adapter_migration(adapter, 1)
    assert adapter == original_adapter
    assert admitted_http == ['adapter-10']
    for original_row, new_row in zip(adapter, migrated):
        if original_row['id'] == 'adapter-10':
            assert new_row['value'] == {'status': 500, 'body': {'error': 'internal'}}
            assert new_row['input'] == original_row['input']
        else:
            assert new_row == original_row
    try:
        adapter_migration(adapter, 0)
    except ValueError:
        pass
    else:
        raise AssertionError('Unexpected HTTP admission count was accepted')
    with gzip.open(root / 'tests/fixtures/events-results.jsonl.gz', 'rt') as stream:
        events = {row['id']: row['value'] for row in map(json.loads, stream)}
    unchanged, admitted = migration_expectations(events, 0)
    assert unchanged == events and not admitted


if __name__ == '__main__':
    test_migration_policy()
    print('Chart migration negative controls passed')
