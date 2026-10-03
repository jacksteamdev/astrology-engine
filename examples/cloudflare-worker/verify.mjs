import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { Miniflare } from 'miniflare';

const { values } = parseArgs({ options: {
  dataset: { type: 'string' }, report: { type: 'string' },
  inputs: { type: 'string' }, results: { type: 'string' },
  adapter: { type: 'string' },
} });
assert.equal(Boolean(values.inputs), Boolean(values.results), '--inputs and --results must be supplied together');
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const parseCapture = text => JSON.parse(text, (_key, value, context) =>
  typeof value === 'number' && Number.isInteger(value) && !Number.isSafeInteger(value)
    ? JSON.rawJSON(context.source)
    : value);
assert.equal(JSON.stringify(parseCapture('{"timestamp":1616302409769941643}')), '{"timestamp":1616302409769941643}');
const options = hash => ({
  modules: true,
  scriptPath: fileURLToPath(new URL('./build/index.js', import.meta.url)),
  modulesRules: [{ type: 'CompiledWasm', include: ['**/*.wasm'], fallthrough: true }],
  compatibilityDate: '2024-11-01',
  compatibilityFlags: [],
  r2Buckets: ['EPHEMERIS'],
  bindings: {
    EPHEMERIS_KEY: 'test/ephemeris.bin', EPHEMERIS_SHA256: hash,
    KERNEL_COVERAGE_MIN_YEAR: '1849', KERNEL_COVERAGE_MAX_YEAR: '2150',
  },
});
const post = (runtime, path, body) => runtime.dispatchFetch(`http://example.test${path}`, {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body),
});
const harness = new Miniflare(options('0'.repeat(64)));
let fixture;
let structural;
try {
  const response = await harness.dispatchFetch('http://example.test/__structural');
  assert.equal(response.status, 200, await response.clone().text());
  structural = await response.json();
  assert.equal(structural.passed, 6);
  fixture = Buffer.from(await (await harness.dispatchFetch('http://example.test/__fixture')).arrayBuffer());
} finally {
  await harness.dispose();
}
const bytes = values.dataset ? await readFile(values.dataset) : fixture;
const runtime = new Miniflare(options(sha256(bytes)));
const request = { date: '2000-01-01T12:00:00Z', location: { latitude: 45, longitude: -90 }, houseSystem: 'placidus' };
const report = {
  runtime: 'wasm32-unknown-unknown/workerd', compatibility_date: '2024-11-01', compatibility_flags: [],
  dataset_sha256: sha256(bytes), structural, adapter_checks: [],
};
try {
  const expectStatus = async (body, status, label) => {
    const response = await post(runtime, '/charts', body);
    assert.equal(response.status, status, `${label}: ${await response.clone().text()}`);
    report.adapter_checks.push(label);
    return response;
  };
  await expectStatus({}, 400, 'malformed request');
  await expectStatus({ ...request, location: { latitude: 91, longitude: 0 } }, 400, 'invalid location');
  await expectStatus({ ...request, date: '1700-01-01T00:00:00Z' }, 422, 'coverage rejection');
  await expectStatus(request, 500, 'absent data');
  const bucket = await runtime.getR2Bucket('EPHEMERIS');
  await bucket.put('test/ephemeris.bin', new Uint8Array([1, 2, 3]));
  await expectStatus(request, 500, 'corrupt data');
  await bucket.put('test/ephemeris.bin', bytes);
  const cold = await expectStatus(request, 200, 'retry after failed initialization');
  assert.equal(cold.headers.get('x-ephemeris-cache'), 'cold');
  const chart = await cold.json();
  assert.equal(chart.planets.length, 19);
  assert.equal(chart.cusps.length, 12);
  await bucket.delete('test/ephemeris.bin');
  const warm = await expectStatus(request, 200, 'warm cache without storage object');
  assert.equal(warm.headers.get('x-ephemeris-cache'), 'warm');
  assert.deepEqual(await warm.json(), chart);
  const whole = await expectStatus({ ...request, houseSystem: 'whole-sign' }, 200, 'whole-sign omits cusps');
  assert.equal(Object.hasOwn(await whole.json(), 'cusps'), false);
  const direct = await post(runtime, '/__capture', [{ id: 'direct', kind: 'chart', date: request.date, lat: 45, lon: -90, house: 'placidus' }]);
  assert.equal(direct.status, 200);
  assert.deepEqual((await direct.json())[0].value.chart, chart);
  report.adapter_checks.push('HTTP payload equals library calculation in Wasm');
  for (const direction of ['forward', 'backward']) {
    const crossing = await post(runtime, '/charts/find-moment', {
      birthMoment: request.date, planet: 'SUN', targetLongitude: values.dataset ? 0 : direction === 'forward' ? 47.01 : 46.99, direction,
    });
    assert.equal(crossing.status, 200, await crossing.clone().text());
    assert.match((await crossing.json()).moment, /T\d{2}:\d{2}:00\.000Z$/);
    report.adapter_checks.push(`${direction} search and minute formatting`);
  }
  if (values.dataset) {
    const response = await runtime.dispatchFetch('http://example.test/__validate');
    assert.equal(response.status, 200, await response.clone().text());
    report.dataset_validation = await response.json();
  }
  if (values.inputs) {
    const inputs = JSON.parse(await readFile(values.inputs, 'utf8'));
    const results = [];
    for (let offset = 0; offset < inputs.length; offset += 50) {
      const response = await post(runtime, '/__capture', inputs.slice(offset, offset + 50));
      assert.equal(response.status, 200, await response.clone().text());
      results.push(...parseCapture(await response.text()));
    }
    assert.equal(results.length, inputs.length);
    await writeFile(values.results, results.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
    report.capture_cases = results.length;
    report.results_sha256 = sha256(await readFile(values.results));
  }
  if (values.adapter) {
    const cases = (await readFile(values.adapter, 'utf8')).trim().split('\n').map(JSON.parse);
    for (const test of cases) {
      const response = await post(runtime, test.input.path, test.input.body);
      assert.deepEqual({ status: response.status, body: await response.json() }, test.value, test.id);
    }
    report.source_adapter_cases = cases.length;
  }
  report.passed = true;
} finally {
  await runtime.dispose();
}
if (values.report) await writeFile(values.report, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
