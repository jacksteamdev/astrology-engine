// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { test, expect } from 'bun:test';
import { downloadVerified, hash, reuseOrInstall, validateVersion, type Download } from './download.ts';
const artifact: Download = { url: 'https://example.invalid/tool.tar.gz', sha256: hash('archive'), version: '1', binary: 'tool', versionOutput: 'tool 1' };
test('transient failure retries, then verifies bytes', async () => {
  let calls = 0; const delays: number[] = [];
  const bytes = await downloadVerified(artifact, async () => ++calls === 1 ? new Response('', { status: 502 }) : new Response('archive'), async ms => { delays.push(ms); });
  expect(new TextDecoder().decode(bytes)).toBe('archive'); expect(calls).toBe(2); expect(delays).toEqual([2000]);
});
test('network errors exhaust exactly three attempts', async () => {
  let calls = 0; const delays: number[] = [];
  await expect(downloadVerified(artifact, async () => { calls++; throw new TypeError('network failure'); }, async ms => { delays.push(ms); })).rejects.toThrow('network failure');
  expect(calls).toBe(3); expect(delays).toEqual([2000, 5000]);
});
test('checksum mismatches and permanent HTTP errors are not retried', async () => {
  for (const response of [new Response('corrupt'), new Response('', { status: 404 })]) {
    let calls = 0;
    await expect(downloadVerified(artifact, async () => { calls++; return response; }, async () => { throw new Error('unexpected retry'); })).rejects.toThrow();
    expect(calls).toBe(1);
  }
});
test('warm installations validate without downloading; wrong versions fail', async () => {
  let installs = 0; let checks = 0;
  await reuseOrInstall(() => true, async () => { installs++; }, () => { checks++; validateVersion('tool 1\n', 'tool 1'); });
  expect(installs).toBe(0); expect(checks).toBe(1);
  await expect(reuseOrInstall(() => true, async () => { installs++; }, () => validateVersion('tool 2', 'tool 1'))).rejects.toThrow('version mismatch');
  expect(installs).toBe(0);
});
