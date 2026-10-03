// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import init, { calculate } from './engine/chart.js';

type Configuration = {reference: string; divisions: string; ophiuchus: string | null};
type Body = {name: string; longitude: number; speed: number; declination: number; sign: {sign: string; degrees_in_sign: number; width_degrees: number}};
type SourceCase = {input: {use_et: boolean; et: number; utc: string; latitude: number; longitude: number; house: string; config: Configuration}; offset: number; bodies: Body[]; cusps: number[]};
type Chart = {reference_offset_degrees: number; bodies: Body[]; cusps: number[]; configuration: Configuration; convention_revision: string; requested_house_system: string; house_system: string; speed_reference: string; zodiac_sectors: {sign: string; width_degrees: number}[]; house_sectors: {sign: string}[] | null};
const datasetPath = process.argv[2];
if (!datasetPath) throw new Error('Usage: bun verify.ts /path/to/captured-cheb.bin [report.json]');
const data = new Uint8Array(await readFile(datasetPath));
const snapshot: {metadata: {dataset_sha256: string; source_revision: string}; charts: SourceCase[]} = JSON.parse(await readFile('../../tests/fixtures/conventions.json', 'utf8'));
const sha256 = createHash('sha256').update(data).digest('hex');
assert.equal(sha256, snapshot.metadata.dataset_sha256, 'Use the source capture dataset');
await init({module_or_path: await readFile('./engine/chart_bg.wasm')});
const maxDelta: Record<string, number> = {};
const compare = (actual: number, expected: number, field: string) => {
  const delta = Math.abs(actual - expected);
  maxDelta[field] = Math.max(maxDelta[field] ?? 0, delta);
  assert.ok(delta <= 1e-12, `${field}: ${actual} differs from ${expected} by ${delta}`);
};
let accepted = 0, rejected = 0;
const requestFor = (c: SourceCase) => ({instant: c.input.use_et ? {scale: 'et', value: c.input.et} : {scale: 'utc', value: c.input.utc}, latitude: c.input.latitude, longitude: c.input.longitude, house_system: c.input.house, configuration: c.input.config});
for (const c of snapshot.charts) {
  const sweep = c.cusps.reduce((sum, cusp, i) => sum + ((c.cusps[(i + 1) % 12] - cusp + 360) % 360), 0);
  const execute = () => JSON.parse(calculate(data, JSON.stringify(requestFor(c)))) as Chart;
  if (Math.abs(sweep - 360) > 1e-8) {
    assert.throws(execute, /overlap/);
    rejected++;
    continue;
  }
  const chart = execute();
  accepted++;
  assert.deepEqual(chart.configuration, c.input.config);
  assert.equal(chart.requested_house_system, c.input.house);
  assert.equal(chart.convention_revision, 'astrology-engine-conventions-v1');
  assert.equal(chart.speed_reference, 'tropical');
  assert.equal(chart.bodies.length, c.bodies.length);
  assert.equal(chart.cusps.length, 12);
  compare(chart.reference_offset_degrees, c.offset, 'offset');
  chart.cusps.forEach((v, i) => compare(v, c.cusps[i], 'cusp'));
  chart.bodies.forEach((v, i) => {
    const expected = c.bodies[i];
    assert.equal(v.name, expected.name);
    assert.equal(v.sign.sign, expected.sign.sign);
    for (const field of ['longitude', 'speed', 'declination'] as const) compare(v[field], expected[field], field);
    compare(v.sign.degrees_in_sign, expected.sign.degrees_in_sign, 'degrees_in_sign');
    compare(v.sign.width_degrees, expected.sign.width_degrees, 'width');
  });
  compare(chart.zodiac_sectors.reduce((sum, sector) => sum + sector.width_degrees, 0), 360, 'sector_sum');
  if (c.input.house === 'whole-sign') {
    assert.equal(chart.house_sectors?.length, 12);
    assert.ok(chart.house_sectors?.every(s => s.sign !== 'ophiuchus'));
  } else { assert.equal(chart.house_sectors, null); }
}
const base = requestFor(snapshot.charts[0]);
const check = (request: unknown) => JSON.parse(calculate(data, JSON.stringify(request))) as Chart;
assert.throws(() => check({...base, latitude: 91}), /latitude/);
assert.throws(() => check({...base, instant: {scale: 'utc', value: '2000-01-01T12:00:00'}}), /explicit UTC/);
assert.throws(() => check({...base, configuration: {reference: 'tropical', divisions: 'constellation', ophiuchus: 'enabled'}}), /require True Sky/);
assert.throws(() => check({...base, configuration: {reference: 'true-sky', divisions: 'constellation', ophiuchus: null}}), /explicit Ophiuchus/);
for (const reference of ['tropical', 'fagan-bradley', 'true-sky']) {
  for (const house_system of ['equal', 'whole-sign', 'placidus']) {
    const result = check({...base, house_system, configuration: {reference, divisions: 'equal', ophiuchus: 'enabled'}});
    assert.equal(result.configuration.ophiuchus, null);
    assert.equal(result.cusps.length, 12);
  }
}
const report = {runtime: 'wasm32-unknown-unknown/Bun', source_revision: snapshot.metadata.source_revision, dataset_sha256: sha256, charts: accepted, rejected_overlap: rejected, tolerance: 1e-12, max_delta: maxDelta};
if (process.argv[3]) await writeFile(process.argv[3], JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
