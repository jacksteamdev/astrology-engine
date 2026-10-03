// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';

export const jobs = ['native', 'tooling', 'worker', 'configurable-chart', 'notices'] as const;
export type Job = typeof jobs[number];
export type Selection = Record<Job, boolean>;
export const allJobs = (): Selection => Object.fromEntries(jobs.map(job => [job, true])) as Selection;
export const selectJobs = (paths: string[], noticeInputs: string[]): Selection => {
  const selected = Object.fromEntries(jobs.map(job => [job, false])) as Selection;
  const inputs = new Set(noticeInputs);
  for (const path of paths) {
    // Legal/build inputs take precedence over documentation exemptions.
    if (inputs.has(path) || /^(LICENSE|NOTICE)(\.|$)/.test(path) ||
        /^(src\/|\.github\/|\.cargo\/|tools\/(ci|notices|verification)\/|notices\/)/.test(path)) return allJobs();
    const prose = /^(docs\/.*\.(md|rst|txt)|[^/]+\.md)$/i.test(path) || /(^|\/)(README|AGENTS)\.md$/.test(path);
    if (prose) continue;
    const affected: Job[] | undefined = path.startsWith('examples/cloudflare-worker/') ? ['worker', 'notices']
      : path.startsWith('examples/configurable-chart/') ? ['configurable-chart', 'notices']
      : /^(tools\/dataset-builder\/|tools\/jpl\/|tools\/regenerate\.py$)/.test(path) ? ['native', 'tooling', 'notices']
      : undefined;
    if (!affected) return allJobs();
    for (const job of affected) selected[job] = true;
  }
  return selected;
};
export const changedPaths = (base: string, head: string, run = (args: string[]): string =>
  execFileSync('git', args, { encoding: 'utf8' })): string[] => {
  if (![base, head].every(ref => /^[a-f0-9]{40}$/.test(ref))) throw new Error('Expected base/head commit IDs');
  const ancestor = run(['merge-base', base, head]).trim();
  // Disable rename collapsing so both old and new paths affect selection.
  return run(['diff', '--name-only', '--no-renames', '-z', ancestor, head, '--']).split('\0').filter(Boolean);
};
export const determineSelection = (event: string, collect: () => { paths: string[]; inputs: string[] }): Selection => {
  if (event !== 'pull_request') return allJobs();
  try {
    const { paths, inputs } = collect();
    return selectJobs(paths, inputs);
  } catch (error) {
    console.warn(`Change detection unavailable; running all checks: ${error}`);
    return allJobs();
  }
};
if (import.meta.main) {
  const selected = determineSelection(process.env.GITHUB_EVENT_NAME ?? '', () => {
    const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH!, 'utf8'));
    const base: string = event.pull_request.base.sha;
    const head: string = event.pull_request.head.sha;
    // Union both versions so deleting a fingerprinted input cannot exempt it.
    const inputs = [base, head].flatMap(ref => Object.keys(JSON.parse(execFileSync('git',
      ['show', `${ref}:notices/inventory.json`], { encoding: 'utf8' })).inputs));
    return { paths: changedPaths(base, head), inputs };
  });
  const output = jobs.map(job => `${job}=${selected[job]}`).join('\n') + '\n';
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, output);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY,
    `### Selected checks\n${jobs.map(job => `- ${job}: ${selected[job] ? 'run' : 'unaffected'}`).join('\n')}\n`);
  console.log(output);
}
