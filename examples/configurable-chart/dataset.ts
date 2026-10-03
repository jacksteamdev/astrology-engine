import type { SegmentIndex } from './prepare-segments.ts';
const fetchBytes = async (url: string) => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Could not load ephemeris: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
};
const cache = new Map<string, Promise<Uint8Array>>();
let indexPromise: Promise<SegmentIndex> | undefined;
export const loadDataset = async (utc: string): Promise<Uint8Array> => {
  const segmented = import.meta.env.VITE_EPHEMERIS_SEGMENTS === 'true';
  if (!segmented) {
    if (!cache.has('full')) cache.set('full', fetchBytes('/ephemeris.bin').catch(e => { cache.delete('full'); throw e; }));
    return cache.get('full')!;
  }
  indexPromise ??= fetch('/ephemeris/index.json').then(async response => {
    if (!response.ok) throw new Error('Could not load segment index');
    const index = await response.json() as SegmentIndex;
    if (index.version !== 1 || index.format !== 'HDCHEB01' || !Number.isFinite(Date.parse(index.birthCoverageUtc?.startUtc)) || !Number.isFinite(Date.parse(index.birthCoverageUtc?.endUtc))) throw new Error('Unsupported segment index');
    return index;
  }).catch(e => { indexPromise = undefined; throw e; });
  const index = await indexPromise;
  const date = new Date(utc);
  if (!utc.endsWith('Z') || !Number.isFinite(date.getTime()) || date.getTime() < Date.parse(index.birthCoverageUtc.startUtc) || date.getTime() > Date.parse(index.birthCoverageUtc.endUtc)) throw new Error('Use a UTC date inside the dataset coverage');
  const segment = index.segments.find(s => s.year === date.getUTCFullYear());
  if (!segment) throw new Error('No segment covers that year');
  let bytes = cache.get(segment.url);
  if (!bytes) {
    bytes = fetchBytes(`/ephemeris/${segment.url}`).then(async data => {
      const digest = await crypto.subtle.digest('SHA-256', data);
      const hash = Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, '0')).join('');
      if (data.length !== segment.bytes || hash !== segment.sha256) throw new Error('Segment integrity check failed');
      return data;
    }).catch(e => { cache.delete(segment.url); throw e; });
  }
  cache.delete(segment.url); cache.set(segment.url, bytes);
  if (cache.size > 3) cache.delete(cache.keys().next().value!);
  return bytes;
};
