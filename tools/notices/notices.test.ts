import { afterEach, test } from 'bun:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, appendFileSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { check, copy, html, inputs, profileNames, read, supplements, write } from './core.ts';
import { validateCollection } from './collect.ts';

const root = resolve(import.meta.dir, '../..');
const temporary: string[] = [];
const fixture = (): string => {
  const out = mkdtempSync(join(tmpdir(), 'engine-notices-'));
  temporary.push(out);
  for (const path of [...Object.keys(inputs(root)), 'notices/inventory.json', ...profileNames.map(p => `notices/${p}.txt`)]) write(out, path, read(root, path));
  return out;
};
afterEach(() => { for (const path of temporary.splice(0)) rmSync(path, { recursive: true, force: true }); });

test('current bundles pass and copy a self-contained notice page', () => {
  const repo = fixture();
  check(repo);
  copy(repo, 'browser', join(repo, 'output'));
  assert.equal(read(repo, 'output/THIRD_PARTY_NOTICES.txt'), read(repo, 'notices/browser.txt'));
  const page = read(repo, 'output/THIRD_PARTY_NOTICES.html');
  assert.match(page, /href="https:\/\/static.crates.io\/crates\/hifitime\/hifitime-4.3.0.crate"/);
  assert.match(page, /Copyright \(c\) Jack Asher/);
});
test('dependency changes cannot silently reuse old notices', () => {
  const repo = fixture();
  appendFileSync(join(repo, 'Cargo.lock'), '\n# changed dependency resolution\n');
  assert.throws(() => check(repo), /stale/);
});
test('missing or altered license material fails closed', () => {
  const repo = fixture();
  appendFileSync(join(repo, 'tools/notices/supplements/anise-LICENSE.txt'), '\nchanged\n');
  assert.throws(() => check(repo), /License material changed/);
  unlinkSync(join(repo, 'tools/notices/supplements/anise-LICENSE.txt'));
  assert.throws(() => check(repo), /ENOENT/);
});
test('truncated notice bundle fails before artifact copying', () => {
  const repo = fixture();
  write(repo, 'notices/browser.txt', 'incomplete');
  assert.throws(() => copy(repo, 'browser', join(repo, 'output')), /Missing or changed/);
});
test('unknown and synthesized license results need explicit review', () => {
  const p = { name: 'unreviewed', version: '1.0.0', license: 'MIT', manifest_path: '/unused/Cargo.toml', source: 'registry+https://github.com/rust-lang/crates.io-index' };
  const data = { crates: [{ package: p, license: 'MIT' }], licenses: [{ id: 'MIT', text: 'license', source_path: null, used_by: [{ crate: p }] }] };
  assert.throws(() => validateCollection(data, new Set()), /Unreviewed synthesized/);
  assert.throws(() => validateCollection({ ...data, licenses: [] }, new Set()), /Unresolved/);
});
test('HTML treats third-party text as text, not executable markup', () => {
  assert.ok(!html('<script>alert("x")</script>').includes('<script>'));
  assert.match(html('https://example.test/source'), /<a href="https:\/\/example.test\/source">/);
});

test('non-crates.io and unreviewed path dependencies cannot acquire misleading notices', () => {
  const p = { name: 'external', version: '1.0.0', license: 'MIT', manifest_path: '/external/Cargo.toml', source: 'registry+https://example.test/index' as string | null };
  const data = { crates: [{ package: p, license: 'MIT' }], licenses: [{ id: 'MIT', text: 'license', source_path: '/external/LICENSE', used_by: [{ crate: p }] }] };
  assert.throws(() => validateCollection(data, new Set()), /unsupported registry/);
  p.source = null;
  assert.throws(() => validateCollection(data, new Set()), /Unreviewed path dependency/);
});
test('upstream supplemental license bytes survive rendering without trimming', () => {
  for (const entry of supplements(root)) for (const profile of entry.profiles) {
    const bundle = read(root, `notices/${profile}.txt`);
    for (const file of entry.files) assert.ok(bundle.includes(read(root, file.path)), `${file.path} must be preserved verbatim in ${profile}`);
  }
});
