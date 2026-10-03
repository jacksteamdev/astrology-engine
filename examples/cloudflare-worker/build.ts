// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { check, copy, workerBuildVersion } from '../../tools/notices/core.ts';

const directory = dirname(fileURLToPath(import.meta.url));
const root = resolve(directory, '../..');
const flags = process.argv.slice(2);
if (flags.length > 1 || flags.some(flag => flag !== '--verification')) throw new Error('Usage: bun build.ts [--verification]');
check(root);
const version = execFileSync('worker-build', ['--version'], { encoding: 'utf8' }).trim().split(/\s+/).at(-1);
if (version !== workerBuildVersion) throw new Error(`Expected worker-build ${workerBuildVersion}, got ${version}`);
execFileSync('worker-build', ['--release', ...(flags.length ? ['--', '--features', 'verification'] : [])], { cwd: directory, stdio: 'inherit' });
copy(root, 'worker', resolve(directory, 'build'));
copy(root, 'worker', resolve(directory, 'build/worker'));
