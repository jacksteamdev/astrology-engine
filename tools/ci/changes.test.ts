// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { test, expect } from 'bun:test';
import { mkdtempSync, writeFileSync, renameSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { allJobs, changedPaths, determineSelection, jobs, selectJobs } from './changes.ts';
import { parseSelection, verifyResults, type Result } from './gate.ts';
const chosen = (paths: string[], inputs: string[] = []) => jobs.filter(job => selectJobs(paths, inputs)[job]);
test('prose skips expensive checks; executable docs and unknown files run all', () => {
  expect(chosen(['README.md', 'docs/architecture.md'])).toEqual([]);
  for (const path of ['docs/demo.ts', 'unknown.config', 'tests/fixtures/regression.txt', 'src/new.rs', '.github/workflows/verify.yml', 'LICENSE', 'NOTICE']) {
    expect(chosen([path])).toEqual([...jobs]);
  }
});
test('gate rejects absent or malformed selection outputs', () => {
  expect(() => parseSelection({})).toThrow('selection');
  const outputs = Object.fromEntries(jobs.map(job => [job, 'true']));
  expect(parseSelection(outputs)).toEqual(allJobs());
  expect(() => parseSelection({ ...outputs, worker: 'tru' })).toThrow('worker');
});
test('adapter and preparation changes select their consumers', () => {
  expect(chosen(['examples/cloudflare-worker/src/lib.rs'])).toEqual(['worker', 'notices']);
  expect(chosen(['examples/configurable-chart/main.ts'])).toEqual(['configurable-chart', 'notices']);
  expect(chosen(['tools/jpl/horizons.py', 'tools/dataset-builder/src/main.rs'])).toEqual(['native', 'tooling', 'notices']);
});
test('notice inputs override prose and narrow adapter selection', () => {
  expect(chosen(['tools/notices/README.md'])).toEqual([...jobs]);
  expect(chosen(['THIRD_PARTY_NOTICES.md'], ['THIRD_PARTY_NOTICES.md'])).toEqual([...jobs]);
  expect(chosen(['examples/cloudflare-worker/Cargo.toml'], ['examples/cloudflare-worker/Cargo.toml'])).toEqual([...jobs]);
});
test('main/manual and unreliable PR comparisons run everything', () => {
  const fail = () => { throw new Error('unavailable base'); };
  expect(determineSelection('push', fail)).toEqual(allJobs());
  expect(determineSelection('workflow_dispatch', fail)).toEqual(allJobs());
  expect(determineSelection('pull_request', fail)).toEqual(allJobs());
});
test('renames and deletions include former paths and use the merge base', () => {
  const dir = mkdtempSync(join(tmpdir(), 'ci-diff-'));
  const git = (args: string[]) => execFileSync('git', args, { cwd: dir, encoding: 'utf8' });
  try {
    git(['init', '-q']); git(['config', 'user.email', 'test@example.invalid']); git(['config', 'user.name', 'CI test']);
    writeFileSync(join(dir, 'old.rs'), 'source'); git(['add', '.']); git(['commit', '-qm', 'base']);
    const base = git(['rev-parse', 'HEAD']).trim();
    renameSync(join(dir, 'old.rs'), join(dir, 'README.md')); git(['add', '-A']); git(['commit', '-qm', 'rename']);
    const head = git(['rev-parse', 'HEAD']).trim();
    expect(changedPaths(base, head, git).sort()).toEqual(['README.md', 'old.rs']);
    expect(chosen(changedPaths(base, head, git))).toEqual([...jobs]);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
test('gate accepts intentional skips, rejects selected skips and every failure/cancellation', () => {
  const selection = selectJobs(['README.md'], []);
  const results: Record<string, Result> = { changes: 'success', 'usage-policy': 'success', ...Object.fromEntries(jobs.map(job => [job, 'skipped' as const])) };
  verifyResults(selection, results);
  expect(() => verifyResults(allJobs(), results)).toThrow();
  for (const result of ['failure', 'cancelled'] as const) expect(() => verifyResults(selection, { ...results, worker: result })).toThrow();
  expect(() => verifyResults(selection, { ...results, changes: 'failure' })).toThrow();
});
