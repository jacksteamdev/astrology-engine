// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { generatorVersion, wasmBindgenVersion, workerBuildVersion } from '../notices/core.ts';
import { downloadVerified, hash, reuseOrInstall, validateVersion, type Download } from './download.ts';
import downloads from './downloads.json';

const groups = ['worker', 'browser', 'notices'] as const;
type Group = typeof groups[number];
const [command, groupArg] = process.argv.slice(2);
if (!groups.includes(groupArg as Group) || !['info', 'install'].includes(command!)) throw new Error('Usage: setup.ts info|install worker|browser|notices');
const group = groupArg as Group;
const root = resolve(import.meta.dir, '../..');
const cache = join(process.env.CI_TOOL_CACHE ?? join(process.env.RUNNER_TEMP ?? '/tmp', 'astrology-ci-tools'), group);
const binaries: Record<string, Download> = group === 'worker' ? downloads : group === 'browser' ? { 'wasm-bindgen': downloads['wasm-bindgen'] } : {};
validateVersion(downloads['wasm-bindgen'].version, wasmBindgenVersion);
const cargoTool = group === 'worker' ? { name: 'worker-build', version: workerBuildVersion, features: [] }
  : group === 'notices' ? { name: 'cargo-about', version: generatorVersion, features: ['--features', 'cli'] } : undefined;
const key = hash(JSON.stringify({ binaries, cargoTool,
  rust: readFileSync(join(root, 'rust-toolchain.toml'), 'utf8'),
  scripts: ['setup.ts', 'download.ts'].map(path => readFileSync(join(import.meta.dir, path), 'utf8')) }));
const output = (file: string | undefined, value: string) => { if (file) appendFileSync(file, value + '\n'); };
if (command === 'info') {
  output(process.env.GITHUB_OUTPUT, `path=${cache}\nkey=ci-tools-v1-${process.platform}-${process.arch}-${group}-${key}`);
  console.log(`Tool cache: ${cache}\nIdentity: ${key}`);
} else {
  if (process.platform !== 'linux' || process.arch !== 'x64') throw new Error('CI helper archives support Linux x64 only. Local builds keep their existing setup.');
  mkdirSync(cache, { recursive: true });
  const binaryPaths: Record<string, string> = {};
  for (const [name, artifact] of Object.entries(binaries)) {
    const destination = join(cache, `${name}-${artifact.version}`);
    const binary = join(destination, artifact.binary);
    await reuseOrInstall(() => existsSync(binary), async () => {
      const staging = mkdtempSync(join(cache, `${name}-download-`));
      try {
        const archive = join(staging, 'archive.tar.gz');
        writeFileSync(archive, await downloadVerified(artifact));
        const extracted = join(staging, 'extracted');
        mkdirSync(extracted);
        execFileSync('tar', ['-xzf', archive, '--strip-components=1', '-C', extracted]);
        validateVersion(execFileSync(join(extracted, artifact.binary), ['--version'], { encoding: 'utf8' }), artifact.versionOutput);
        renameSync(extracted, destination);
      } finally { rmSync(staging, { recursive: true, force: true }); }
    }, () => validateVersion(execFileSync(binary, ['--version'], { encoding: 'utf8' }), artifact.versionOutput));
    binaryPaths[name] = binary;
    console.log(`Validated ${name} ${artifact.version}`);
  }
  if (cargoTool) {
    const destination = join(cache, `${cargoTool.name}-${cargoTool.version}`);
    const binary = join(destination, 'bin', cargoTool.name);
    const expected = cargoTool.name === 'worker-build' ? cargoTool.version : `${cargoTool.name} ${cargoTool.version}`;
    await reuseOrInstall(() => existsSync(binary), async () => {
      const staging = mkdtempSync(join(cache, 'cargo-install-'));
      try {
        execFileSync('cargo', ['install', cargoTool.name, '--bin', cargoTool.name, '--version', cargoTool.version,
          '--locked', '--root', staging, ...cargoTool.features], { stdio: 'inherit', env: { ...process.env, CARGO_NET_RETRY: '3' } });
        validateVersion(execFileSync(join(staging, 'bin', cargoTool.name), ['--version'], { encoding: 'utf8' }), expected);
        renameSync(staging, destination);
      } finally { rmSync(staging, { recursive: true, force: true }); }
    }, () => validateVersion(execFileSync(binary, ['--version'], { encoding: 'utf8' }), expected));
    output(process.env.GITHUB_PATH, dirname(binary));
    if (cargoTool.name === 'cargo-about') output(process.env.GITHUB_ENV, `CARGO_ABOUT=${binary}`);
    console.log(`Validated ${cargoTool.name} ${cargoTool.version}`);
  }
  for (const [name, binary] of Object.entries(binaryPaths)) {
    output(process.env.GITHUB_ENV, `${name.toUpperCase().replaceAll('-', '_')}_BIN=${binary}`);
    if (name === 'wasm-bindgen') output(process.env.GITHUB_ENV, `WASM_BINDGEN=${binary}`);
  }
}
