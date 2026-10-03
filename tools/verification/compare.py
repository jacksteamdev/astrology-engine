# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

import argparse
import json
from pathlib import Path


def load(path):
    rows = [json.loads(line) for line in Path(path).read_text().splitlines() if line]
    indexed = {row['id']: row['value'] for row in rows}
    if len(indexed) != len(rows):
        raise ValueError('Duplicate result identifiers')
    return indexed


def compare(expected, actual):
    missing = sorted(expected.keys() - actual.keys())
    extra = sorted(actual.keys() - expected.keys())
    different = sorted(key for key in expected.keys() & actual.keys() if expected[key] != actual[key])
    return {'passed': not (missing or extra or different), 'expected_count': len(expected),
            'actual_count': len(actual), 'missing': missing, 'extra': extra, 'different': different}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('expected', type=Path)
    parser.add_argument('actual', type=Path)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    result = compare(load(args.expected), load(args.actual))
    text = json.dumps(result, indent=2) + '\n'
    if args.report:
        args.report.write_text(text)
    print(text, end='')
    raise SystemExit(0 if result['passed'] else 1)


if __name__ == '__main__':
    main()
