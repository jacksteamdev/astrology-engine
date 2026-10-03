import { check, copy, wasmBindgenVersion } from '../../tools/notices/core.ts';
import { resolve } from 'node:path';

const root = resolve(import.meta.dir, '../..');
check(root);
const run = (cmd: string[]) => {
  const result = Bun.spawnSync(cmd, {stdout: 'inherit', stderr: 'inherit'});
  if (result.exitCode !== 0) throw new Error(`Command failed: ${cmd.join(' ')}`);
};
const bindgen = process.env.WASM_BINDGEN ?? 'wasm-bindgen';
const version = Bun.spawnSync([bindgen, '--version']);
if (version.exitCode !== 0 || version.stdout.toString().trim() !== `wasm-bindgen ${wasmBindgenVersion}`) throw new Error(`Expected wasm-bindgen ${wasmBindgenVersion}`);
run(['cargo', 'build', '--locked', '--release', '--target', 'wasm32-unknown-unknown', '--manifest-path', 'wasm/Cargo.toml']);
run([bindgen, 'wasm/target/wasm32-unknown-unknown/release/configurable_chart_example.wasm', '--target', 'web', '--out-dir', 'engine', '--out-name', 'chart']);

copy(root, 'browser', resolve('engine'));
copy(root, 'browser', resolve('public'));
