// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import init, { calculate } from './engine/chart.js';

const form = document.querySelector<HTMLFormElement>('#chart-form')!;
const output = document.querySelector<HTMLElement>('#result')!;
const status = document.querySelector<HTMLElement>('#status')!;
const reference = form.elements.namedItem('reference') as HTMLSelectElement;
const divisions = form.elements.namedItem('divisions') as HTMLSelectElement;
const ophiuchus = form.elements.namedItem('ophiuchus') as HTMLSelectElement;
const syncControls = () => {
  divisions.disabled = reference.value !== 'true-sky';
  if (divisions.disabled) divisions.value = 'equal';
  ophiuchus.disabled = divisions.value !== 'constellation';
};
reference.addEventListener('change', syncControls);
divisions.addEventListener('change', () => { ophiuchus.value = 'enabled'; syncControls(); });
syncControls();
let datasetBytes: Uint8Array | undefined;
const loadDataset = async (): Promise<Uint8Array> => {
  if (datasetBytes) return datasetBytes;
  const response = await fetch('/ephemeris.bin');
  if (!response.ok) throw new Error(`Could not load the ephemeris: ${await response.text()}`);
  datasetBytes = new Uint8Array(await response.arrayBuffer());
  return datasetBytes;
};
form.addEventListener('submit', async event => {
  event.preventDefault();
  const button = form.querySelector('button')!;
  button.disabled = true;
  status.textContent = 'Calculating…';
  output.textContent = '';
  try {
    status.textContent = 'Loading ephemeris…';
    const [, bytes] = await Promise.all([init(), loadDataset()]);
    status.textContent = 'Calculating…';
    const fields = new FormData(form);
    const request = {
      instant: {scale: 'utc', value: String(fields.get('utc'))},
      latitude: Number(fields.get('latitude')), longitude: Number(fields.get('longitude')),
      house_system: String(fields.get('houses')),
      configuration: {reference: reference.value, divisions: divisions.value, ophiuchus: ophiuchus.disabled ? null : ophiuchus.value},
    };
    const chart = JSON.parse(calculate(bytes, JSON.stringify(request)));
    output.textContent = JSON.stringify(chart, null, 2);
    status.textContent = `Calculated ${chart.bodies.length} bodies and angles, with ${chart.house_system} houses.`;
  } catch (error) {
    status.textContent = String(error);
  } finally { button.disabled = false; }
});
