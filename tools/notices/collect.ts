// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { filesBelow, generatorVersion, profiles, read, sha256, supplements, type PackageNotice, type Profile } from './core.ts';

type Crate = { name: string; version: string; source: string | null; manifest_path: string; license: string | null };
type License = { id: string; text: string; source_path: string | null; used_by: { crate: Crate }[] };
type Collected = { crates: { package: Crate; license: string }[]; licenses: License[] };
const sourceUrl = (p: Crate): string => `https://static.crates.io/crates/${p.name}/${p.name}-${p.version}.crate`;
const key = (p: { name: string; version: string }): string => `${p.name}@${p.version}`;
const legalFile = (name: string): boolean => /^(licen[cs]e|copying|notice|copyright|authors)([._-]|$)/i.test(name);
const legalFiles = (directory: string): string[] => {
  const walk = (dir: string): string[] => readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const path = join(dir, entry.name);
    if (entry.isFile() && legalFile(entry.name)) return [path];
    if (entry.isDirectory() && !['.git', 'target', 'tests', 'benches', 'examples'].includes(entry.name)) return walk(path);
    return [];
  });
  return walk(directory).sort();
};
export const validateCollection = (data: Collected, reviewed: Set<string>, firstPartyManifests = new Set<string>()): void => {
  for (const { package: p, license } of data.crates) {
    if (!p.source) {
      if (!firstPartyManifests.has(p.manifest_path) || p.license !== 'MPL-2.0') throw new Error(`Unreviewed path dependency: ${key(p)}`);
      continue;
    }
    if (p.source !== 'registry+https://github.com/rust-lang/crates.io-index') throw new Error(`Review unsupported registry/source: ${key(p)}`);
    const found = data.licenses.filter(l => l.used_by.some(u => key(u.crate) === key(p)));
    if (!p.license || ['Unknown', 'Ignore'].includes(license) || found.length === 0 || found.some(l => !l.text.trim())) {
      throw new Error(`Unresolved license: ${key(p)}`);
    }
    if (found.some(license => !license.source_path) && !reviewed.has(key(p))) {
      throw new Error(`Unreviewed synthesized license text: ${key(p)}`);
    }
  }
};
export const collect = (root: string, profile: Profile): { text: string; packages: PackageNotice[] } => {
  const binary = process.env.CARGO_ABOUT ?? 'cargo-about';
  const version = execFileSync(binary, ['--version'], { encoding: 'utf8' }).trim();
  if (version !== `cargo-about ${generatorVersion}`) throw new Error(`Expected cargo-about ${generatorVersion}; got ${version}`);
  const p = profiles[profile];
  const args = ['generate', '--locked', '--offline', '--fail', '--format', 'json', '--manifest-path', p.manifest, '--config', 'tools/notices/about.toml'];
  if (p.target) args.push('--target', p.target);
  const data: Collected = JSON.parse(execFileSync(binary, args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] }));
  const extra = supplements(root).filter(s => s.profiles.includes(profile));
  for (const entry of extra) {
    if (entry.name === 'anise' && !data.crates.some(c => key(c.package) === key(entry))) throw new Error('Update ANISE supplemental attribution for the resolved version.');
    if (entry.name === 'wasm-bindgen-cli-support' && !data.crates.some(c => c.package.name === 'wasm-bindgen' && c.package.version === entry.version)) throw new Error('Update Wasm generator attribution for the resolved version.');
  }
  const fileMap = new Map(data.crates.filter(c => c.package.source).map(c => [key(c.package), legalFiles(dirname(c.package.manifest_path))]));
  const reviewed: { name: string; version: string; path: string; sha256: string }[] = JSON.parse(read(root, 'tools/notices/reviewed-files.json'));
  const verified = new Set(extra.map(key));
  for (const item of reviewed) {
    const entry = data.crates.find(c => key(c.package) === key(item));
    if (!entry) continue;
    const text = readFileSync(join(dirname(entry.package.manifest_path), item.path), 'utf8');
    if (sha256(text) !== item.sha256) throw new Error(`Reviewed license changed: ${key(item)}/${item.path}`);
    verified.add(key(item));
  }
  const firstPartyManifests = new Set([
    'Cargo.toml', 'tools/dataset-builder/Cargo.toml',
    'examples/cloudflare-worker/Cargo.toml', 'examples/configurable-chart/wasm/Cargo.toml',
  ].map(path => join(root, path)));
  validateCollection(data, verified, firstPartyManifests);
  const packages = data.crates.filter(c => c.package.source).map(({ package: p }) => ({ name: p.name, version: p.version, license: p.license!, source: sourceUrl(p) })).sort((a, b) => key(a).localeCompare(key(b), 'en'));
  const texts = new Map<string, { text: string; labels: Set<string> }>();
  const add = (text: string, label: string) => {
    if (!text.trim()) throw new Error(`Empty license material: ${label}`);
    const hash = sha256(text);
    const current = texts.get(hash) ?? { text, labels: new Set<string>() };
    current.labels.add(label);
    texts.set(hash, current);
  };
  for (const license of data.licenses) {
    const users = license.used_by.filter(u => u.crate.source);
    if (users.length) add(license.text, `${license.id}: ${users.map(u => key(u.crate)).sort().join(', ')}`);
  }
  for (const { package: p } of data.crates) {
    if (!p.source) continue;
    const dir = dirname(p.manifest_path);
    for (const file of fileMap.get(key(p)) ?? []) add(readFileSync(file, 'utf8'), `${key(p)} — ${relative(dir, file)}`);
    // libm carries per-routine grants; MPL crates carry copyright headers that
    // their standalone license text does not contain. Retain the comment blocks.
    if (['libm', 'hifitime', 'anise'].includes(p.name) && existsSync(join(dir, 'src'))) {
      for (const file of filesBelow(dir, 'src').filter(f => f.endsWith('.rs'))) {
        const source = readFileSync(join(dir, file), 'utf8');
        const blocks = source.match(/\/\*[\s\S]*?\*\/|(?:^[ \t]*\/\/[^\n]*(?:\n|$))+/gm) ?? [];
        for (const block of blocks) if (/copyright|permission is hereby|permission to use/i.test(block)) add(block, `${key(p)} — ${file} source notice`);
      }
    }
  }
  for (const entry of extra) for (const file of entry.files) add(read(root, file.path), `${key(entry)} — ${file.path.split('/').at(-1)}\nSource: ${entry.source}`);
  const header = [
    `Astrology Engine — ${profile} distribution notices`,
    'Generated by cargo-about 0.9.2 and tools/notices/notices.ts. Do not edit this bundle.',
    'This inventory conservatively includes build-time Rust dependencies as well as runtime dependencies; it does not claim every listed package contributes code to the executable.',
    'Astrology Engine and the local adapters are MPL-2.0 licensed. Third-party components retain their own terms.',
    'MPL source access: obtain each unmodified covered dependency from its exact-version source URL below. If distributing modifications, provide their corresponding MPL source and preserve these notices. Keep source available to recipients.',
    'The project does not modify the dependency source listed here.',
    profile === 'builder' ? 'SOFA attribution: through the unchanged sofars Rust port used by ANISE, this builder uses routines and computations derived from software provided by the IAU SOFA Board. The Rust port and this project are not software provided by or endorsed by SOFA. We use the port as a dependency and make no changes to its routines. Its complete upstream terms are reproduced below.' : '',
    'First-party source: ASTROLOGY_ENGINE_SOURCE.tar.gz accompanies built distributions. It contains the project source captured when the artifact is built. Preserve or provide this archive to recipients.',
    '\nProject copyright and license notice\n' + read(root, 'NOTICE'),
    '\nProject license\n' + read(root, 'LICENSE'),
    '\nCopied/adapted Astronomy Engine code\n' + read(root, 'tools/notices/supplements/astronomy-engine-MIT.txt'),
    'Rust 1.96.0 standard-library notices accompany this bundle in RUST_LIBRARY_NOTICES.html. Upstream: https://doc.rust-lang.org/1.96.0/COPYRIGHT-library.html',
    '\nDependency source index',
    ...packages.map(p => `${key(p)} — ${p.license}\n${p.source}`),
    ...extra.map(p => `${key(p)} — additional attribution for upstream notices or emitted JavaScript\n${p.source}`),
    '\nLicense and copyright texts (including upstream alternatives and notices)',
  ].filter(Boolean).join('\n\n');
  const sections = [...texts.values()].map(t => ({ ...t, heading: [...t.labels].sort().join('\n') })).sort((a,b) => a.heading.localeCompare(b.heading, 'en'));
  return { packages, text: header + '\n\n' + sections.map(s => `${'='.repeat(72)}\n${s.heading}\n${'='.repeat(72)}\n\n${s.text}\n`).join('\n') };
};
