import { execFileSync } from 'node:child_process';
import { mkdir, rename, rm, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';

type Span = { lo: string; hi: string };
type Manifest = { format: 'HDCHEB01'; bytes: number; sha256: string; sourceSha256?: string; domainUtc: Span; fittedUtc: Span; domainEt: {lo: number; hi: number} };
export type SegmentIndex = {
  version: 1; format: 'HDCHEB01'; source: Manifest;
  birthCoverageUtc: {startUtc: string; endUtc: string};
  segments: {year: number; url: string; bytes: number; sha256: string; domainUtc: Span; fittedUtc: Span}[];
};
// Build-time policy: annual starts, two-year windows, one public day of padding.
// The Rust extractor owns binary layout, ET conversion and fitted guard margins.
export const prepareSegments = async (input: string, output: string, extractor: string): Promise<SegmentIndex> => {
  const invoke = (args: string[]): Manifest => JSON.parse(execFileSync(extractor, args, {encoding: 'utf8'}));
  const source = invoke(['inspect', '--input', input]);
  const start = Date.parse(source.domainUtc.lo), end = Date.parse(source.domainUtc.hi);
  if (!Number.isFinite(start) || !Number.isFinite(end)) throw new Error('Invalid source UTC coverage');
  const segments: SegmentIndex['segments'] = [];
  await mkdir(join(output, source.sha256), {recursive: true});
  for (let year = new Date(start).getUTCFullYear(); year <= new Date(end).getUTCFullYear(); year++) {
    const boundary = (y: number) => { const d = new Date(0); d.setUTCFullYear(y, 0, 1); return d.getTime(); };
    const lo = boundary(year - 1) - 86_400_000, hi = boundary(year + 1) + 86_400_000;
    const bin = join(output, source.sha256, `${year}.bin`), meta = `${bin}.json`;
    const manifest = invoke(['extract', '--input', input, '--start-utc', lo <= start ? source.domainUtc.lo : new Date(lo).toISOString(), '--end-utc', hi >= end ? source.domainUtc.hi : new Date(hi).toISOString(), '--out', bin, '--manifest', meta]);
    const url = `${source.sha256}/${manifest.sha256}.bin`;
    await rename(bin, join(output, url));
    await rm(meta);
    segments.push({year, url, bytes: manifest.bytes, sha256: manifest.sha256, domainUtc: manifest.domainUtc, fittedUtc: manifest.fittedUtc});
  }
  const index: SegmentIndex = {version: 1, format: 'HDCHEB01', source, birthCoverageUtc: {startUtc: source.domainUtc.lo, endUtc: source.domainUtc.hi}, segments};
  await writeFile(join(output, 'index.json'), JSON.stringify(index, null, 2) + '\n');
  return index;
};
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [input, output] = process.argv.slice(2);
  if (!input || !output) throw new Error('Usage: bun prepare-segments.ts /path/to/cheb.bin /new/output/directory');
  // An existing output directory is intentionally not cleaned by this example.
  const index = await prepareSegments(resolve(input), resolve(output), process.env.EPHEMERIS_EXTRACTOR ?? '../../target/release/dataset-builder');
  console.log(JSON.stringify({segments: index.segments.length, sourceBytes: index.source.bytes, totalBytes: index.segments.reduce((n, s) => n + s.bytes, 0)}));
}
