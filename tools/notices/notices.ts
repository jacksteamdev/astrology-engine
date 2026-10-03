import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { check, copy, generatorVersion, inputs, profileNames, read, sha256, write, type Inventory, type Profile } from './core.ts';
import { collect } from './collect.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const [command, name, destination] = process.argv.slice(2);
const profile = (name: string | undefined): Profile => {
  if (!profileNames.includes(name as Profile)) throw new Error(`Choose a profile: ${profileNames.join(', ')}`);
  return name as Profile;
};
if (command === 'update' || command === 'verify') {
  const results = Object.fromEntries(profileNames.map(p => [p, collect(root, p)])) as Record<Profile, ReturnType<typeof collect>>;
  const inventory: Inventory = {
    generator: `cargo-about ${generatorVersion}`, inputs: inputs(root),
    profiles: Object.fromEntries(profileNames.map(p => [p, { packages: results[p].packages, sha256: sha256(results[p].text) }])) as Inventory['profiles'],
  };
  const outputs = { ...Object.fromEntries(profileNames.map(p => [`notices/${p}.txt`, results[p].text])), 'notices/inventory.json': JSON.stringify(inventory, null, 2) + '\n' };
  for (const [file, text] of Object.entries(outputs)) {
    if (command === 'update') write(root, file, text);
    else if (read(root, file) !== text) throw new Error(`Generated notice drift: ${file}`);
  }
} else if (command === 'check') {
  check(root);
} else if (command === 'copy') {
  if (!destination) throw new Error('Usage: notices.ts copy PROFILE OUTPUT_DIRECTORY');
  copy(root, profile(name), destination);
} else if (command === 'package-builder') {
  check(root);
  const out = join(root, 'target/distribution/dataset-builder');
  execFileSync('cargo', ['build', '--locked', '--release', '-p', 'dataset-builder'], { cwd: root, stdio: 'inherit', env: { ...process.env, CARGO_TARGET_DIR: join(root, 'target') } });
  const executable = process.platform === 'win32' ? 'dataset-builder.exe' : 'dataset-builder';
  mkdirSync(out, { recursive: true });
  copyFileSync(join(root, 'target/release', executable), join(out, executable));
  copy(root, 'builder', out);
} else if (command === 'check-sources') {
  const inventory = check(root);
  const urls = new Set(Object.values(inventory.profiles).flatMap(p => p.packages.filter(p => p.license.includes('MPL-2.0')).map(p => p.source)));
  for (const url of urls) {
    const response = await fetch(url);
    if (!response.ok || (await response.arrayBuffer()).byteLength === 0) throw new Error(`Source download unavailable: ${url}`);
    console.log(`Source download available: ${url}`);
  }
} else {
  throw new Error('Usage: bun tools/notices/notices.ts update|verify|check|copy PROFILE OUTPUT_DIRECTORY|package-builder|check-sources');
}
console.log(`Notices: ${command} passed`);
