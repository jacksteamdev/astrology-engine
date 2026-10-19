// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import init, { calculate } from './engine/chart.js';

const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');
const compare = (actual: unknown, expected: unknown): void => {
  if (typeof actual === 'number' && typeof expected === 'number') {
    assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) <= 1e-12, `${actual} != ${expected}`);
  } else if (actual !== null && expected !== null && typeof actual === 'object' && typeof expected === 'object') {
    assert.equal(Array.isArray(actual), Array.isArray(expected));
    if (Array.isArray(actual) && Array.isArray(expected)) assert.equal(actual.length, expected.length);
    assert.deepEqual(Object.keys(actual).sort(), Object.keys(expected).sort());
    for (const key of Object.keys(expected)) compare((actual as Record<string, unknown>)[key], (expected as Record<string, unknown>)[key]);
  } else assert.equal(actual, expected);
};
compare(29.288207831506444, 29.288207831506437);
assert.throws(() => compare(1, 1.00000001));
assert.throws(() => compare({'0': 1}, [1]));
assert.throws(() => compare(['SUN', 'MOON'], ['MOON', 'SUN']));
const path = process.argv[2];
if (!path) throw new Error('Usage: bun verify-fagan-bradley.ts /path/to/historical-cheb.bin');
const fixture = await readFile(new URL('../../tests/fixtures/fagan-bradley-71350f0.json', import.meta.url));
assert.equal(hash(fixture), 'd70c88d0452efce0f0725a8baf8109992915b4221beed2ec1d75840bf3eb3c8a');
const source = JSON.parse(fixture.toString()) as {engineRevision: string; datasetSha256: string; cases: {input: [string, number, number, string]; chart: unknown}[]};
assert.equal(source.engineRevision, '71350f0889c3b5e1585935d2017c97efe0bb5848');
assert.equal(source.cases.length, 8);
const bytes = await readFile(path);
assert.equal(hash(bytes), source.datasetSha256, 'Use the historical dataset, not the conventions dataset');
await init({module_or_path: await readFile(new URL('./engine/chart_bg.wasm', import.meta.url))});
for (const {input: [utc, latitude, longitude, house_system], chart: expected} of source.cases) {
  const chart = JSON.parse(calculate(bytes, JSON.stringify({instant: {scale:'utc', value:utc}, latitude, longitude, house_system, configuration: {reference:'fagan-bradley', divisions:'equal', ophiuchus:null}}))) as {bodies: {name: string; longitude: number; speed: number; declination: number}[]; cusps: number[]; reference_offset_degrees: number; house_system: string};
  compare({bodies: chart.bodies.map(({name, longitude, speed, declination}) => ({name, longitude, speed, declination})), cusps: chart.cusps, ayanamsa_degrees: chart.reference_offset_degrees, house_system: chart.house_system}, expected);
}
console.log(JSON.stringify({runtime:'Wasm', cases:8, datasetSha256:source.datasetSha256, tolerance:1e-12}));
