// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { jobs, type Selection } from './changes.ts';
export type Result = 'success' | 'failure' | 'cancelled' | 'skipped';
export const parseSelection = (outputs: Record<string, unknown>): Selection => {
  for (const job of jobs) {
    if (outputs[job] !== 'true' && outputs[job] !== 'false') throw new Error(`Missing or invalid selection: ${job}`);
  }
  return Object.fromEntries(jobs.map(job => [job, outputs[job] === 'true'])) as Selection;
};
export const verifyResults = (selection: Selection, results: Record<string, Result>): void => {
  for (const job of ['changes', 'usage-policy', ...jobs]) {
    const required = job === 'changes' || job === 'usage-policy' || selection[job as keyof Selection];
    const result = results[job];
    if (result !== 'success' && !(result === 'skipped' && !required)) throw new Error(`${job}: ${result ?? 'missing result'}`);
  }
};
if (import.meta.main) {
  const needs = JSON.parse(process.env.CI_NEEDS!);
  const selection = parseSelection(needs.changes?.outputs ?? {});
  verifyResults(selection, Object.fromEntries(Object.entries(needs).map(([name, job]) => [name, (job as { result: Result }).result])));
  console.log('All selected verification jobs passed.');
}
