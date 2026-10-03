// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { createHash } from 'node:crypto';
export type Download = { url: string; sha256: string; version: string; binary: string; versionOutput: string };
type Fetch = (url: string, init: RequestInit) => Promise<Response>;
export const hash = (bytes: string | Uint8Array): string => createHash('sha256').update(bytes).digest('hex');
export const downloadVerified = async (artifact: Download, fetcher: Fetch = fetch,
  sleep: (ms: number) => Promise<void> = Bun.sleep): Promise<Uint8Array> => {
  for (let attempt = 0; attempt < 3; attempt++) {
    let bytes: Uint8Array | undefined;
    let failure: unknown;
    let transient = false;
    try {
      const response = await fetcher(artifact.url, { signal: AbortSignal.timeout(60_000) });
      if (response.ok) bytes = new Uint8Array(await response.arrayBuffer());
      else {
        failure = new Error(`HTTP ${response.status}: ${artifact.url}`);
        transient = response.status === 408 || response.status === 429 || response.status >= 500;
        await response.body?.cancel();
      }
    } catch (error) {
      failure = error;
      transient = true; // Fetch/response transport failed; verification is outside this block.
    }
    if (bytes) {
      if (hash(bytes) !== artifact.sha256) throw new Error(`Checksum mismatch: ${artifact.url}`);
      return bytes;
    }
    if (!transient || attempt === 2) throw failure;
    console.warn(`Tool download attempt ${attempt + 1} failed: ${failure}`);
    await sleep([2_000, 5_000][attempt]!);
  }
  throw new Error('Download attempts exhausted');
};
export const validateVersion = (actual: string, expected: string): void => {
  if (actual.trim() !== expected) throw new Error(`Tool version mismatch: expected ${expected}, got ${actual.trim()}`);
};
export const reuseOrInstall = async (exists: () => boolean, install: () => Promise<void>, validate: () => void): Promise<void> => {
  if (!exists()) await install();
  validate();
};
