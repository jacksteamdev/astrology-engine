const run = (cmd: string[]) => {
  const result = Bun.spawnSync(cmd, {stdout: 'inherit', stderr: 'inherit'});
  if (result.exitCode !== 0) throw new Error(`Command failed: ${cmd.join(' ')}`);
};
run(['cargo', 'build', '--locked', '--release', '--target', 'wasm32-unknown-unknown', '--manifest-path', 'wasm/Cargo.toml']);
run([process.env.WASM_BINDGEN ?? 'wasm-bindgen', 'wasm/target/wasm32-unknown-unknown/release/configurable_chart_example.wasm', '--target', 'web', '--out-dir', 'engine', '--out-name', 'chart']);
