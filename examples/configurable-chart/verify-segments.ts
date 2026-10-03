import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import init, {calculate, design_moment_et} from './engine/chart.js';
import type {SegmentIndex} from './prepare-segments.ts';
const [sourcePath, directory] = process.argv.slice(2);
if (!sourcePath || !directory) throw new Error('Usage: bun verify-segments.ts /source/cheb.bin /segments');
const source = new Uint8Array(await readFile(sourcePath));
const index: SegmentIndex = JSON.parse(await readFile(resolve(directory, 'index.json'), 'utf8'));
assert.equal(createHash('sha256').update(source).digest('hex'), index.source.sha256);
await init({module_or_path: await readFile(new URL('./engine/chart_bg.wasm', import.meta.url))});
let charts = 0, designs = 0, sourceBoundaryRejections = 0, maximumDifference = 0;
const compare = (a: unknown, b: unknown): void => {
  if (typeof a === 'number' && typeof b === 'number') {
    const delta = Math.abs(a-b); maximumDifference = Math.max(maximumDifference, delta);
    assert.ok(delta <= 1e-12, `Numeric difference ${delta}`);
  } else if (a !== null && b !== null && typeof a === 'object' && typeof b === 'object') {
    assert.deepEqual(Object.keys(a), Object.keys(b));
    for (const key of Object.keys(a)) compare((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]);
  } else assert.equal(a,b);
};
for (const segment of index.segments) {
  const bytes = new Uint8Array(await readFile(resolve(directory,segment.url)));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), segment.sha256);
  const y = segment.year;
  const dates = [`${y}-01-01T00:00:00Z`, `${y}-03-21T12:00:00Z`, `${y}-06-21T12:00:00Z`, `${y}-09-21T12:00:00Z`, `${y}-12-31T23:59:00Z`];
  if (new Date(`${y}-02-29T12:00:00Z`).getUTCMonth() === 1) dates.push(`${y}-02-29T12:00:00Z`);
  for (const utc of dates) {
    if (Date.parse(utc) < Date.parse(index.birthCoverageUtc.startUtc) || Date.parse(utc) > Date.parse(index.birthCoverageUtc.endUtc)) continue;
    for (const [reference, divisions, ophiuchus] of [['tropical','equal',null], ['fagan-bradley','equal',null], ['true-sky','constellation','enabled']] as const) {
      const request = {instant:{scale:'utc', value:utc},latitude:51.5,longitude:-0.12,house_system:['equal','whole-sign','placidus'][charts%3],configuration:{reference,divisions,ophiuchus}};
      compare(JSON.parse(calculate(source,JSON.stringify(request))), JSON.parse(calculate(bytes,JSON.stringify(request)))); charts++;
    }
    let full: number;
    try { full = design_moment_et(source,utc); }
    catch (error) {
      assert.match(String(error), /outside coverage/, 'Unexpected design-search failure');
      assert.ok(Date.parse(utc) - Date.parse(index.source.domainUtc.lo) < 89 * 86_400_000 + 2000, 'Only source-boundary admission failures are expected');
      assert.throws(() => design_moment_et(bytes,utc), /outside coverage/);
      sourceBoundaryRejections++; continue;
    }
    assert.equal(design_moment_et(bytes,utc),full);
    const request = {instant:{scale:'et',value:full},latitude:51.5,longitude:-0.12,house_system:'equal',configuration:{reference:'tropical',divisions:'equal',ophiuchus:null}};
    compare(JSON.parse(calculate(source,JSON.stringify(request))),JSON.parse(calculate(bytes,JSON.stringify(request)))); designs++;
  }
}
console.log(JSON.stringify({runtime:'Wasm',sourceSha256:index.source.sha256,segments:index.segments.length,charts,designs,sourceBoundaryRejections,maximumDifference,tolerance:1e-12},null,2));
