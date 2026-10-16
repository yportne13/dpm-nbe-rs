#!/usr/bin/env node
// Build the browser language server.
//
//   node web/build.mjs            # release build (what `serve.mjs` serves)
//   node web/build.mjs --debug    # faster rebuild, bigger wasm
//
// Two steps, and both are required:
//   1. cargo build --target wasm32-unknown-unknown
//   2. wasm-bindgen --target web, which writes the JS glue next to the .wasm
//
// The `wasm-bindgen` CLI must be exactly the version pinned in
// `web/api/dpm-nbe-wasm/Cargo.toml` (=0.2.128); a mismatch produces glue that
// cannot instantiate.  `rustup target add wasm32-unknown-unknown` is the other
// prerequisite.

import { spawnSync } from 'node:child_process';
import { existsSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const WEB = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(WEB, '..');
const API = path.join(WEB, 'api', 'dpm-nbe-wasm');
const MANIFEST = path.join(API, 'Cargo.toml');
const OUT_DIR = path.join(WEB, 'www', 'pkg', 'dpm-nbe-wasm');
const OUT_NAME = 'dpm_nbe_wasm';

const debug = process.argv.includes('--debug');
const profile = debug ? 'debug' : 'release';
const wasmPath = path.join(
  API,
  'target',
  'wasm32-unknown-unknown',
  profile,
  `${OUT_NAME}.wasm`,
);

function run(command, args, options = {}) {
  process.stdout.write(`\n$ ${command} ${args.join(' ')}\n`);
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    cwd: options.cwd ?? REPO,
    // `cargo` and `wasm-bindgen` are real executables; no shell is involved, so
    // arguments cannot be mangled by one.
    shell: false,
  });
  if (result.error) {
    console.error(`cannot run ${command}: ${result.error.message}`);
    process.exit(2);
  }
  if (result.status !== 0) {
    console.error(`${command} failed with exit code ${result.status}`);
    process.exit(result.status ?? 1);
  }
}

run('cargo', [
  'build',
  ...(debug ? [] : ['--release']),
  '--target',
  'wasm32-unknown-unknown',
  '--manifest-path',
  MANIFEST,
]);

if (!existsSync(wasmPath)) {
  console.error(`expected ${wasmPath} after the cargo build`);
  process.exit(2);
}

run('wasm-bindgen', [
  '--target',
  'web',
  '--out-dir',
  OUT_DIR,
  '--out-name',
  OUT_NAME,
  wasmPath,
]);

const artifacts = ['dpm_nbe_wasm.js', 'dpm_nbe_wasm_bg.wasm'];
for (const file of artifacts) {
  const full = path.join(OUT_DIR, file);
  if (!existsSync(full)) {
    console.error(`wasm-bindgen did not produce ${full}`);
    process.exit(2);
  }
  process.stdout.write(`  ${file}: ${statSync(full).size} bytes\n`);
}

// The glue is an ES module; say so, or Node guesses (and warns) when the smoke
// test imports it directly from disk.
writeFileSync(
  path.join(OUT_DIR, 'package.json'),
  `${JSON.stringify({ type: 'module', private: true }, null, 2)}\n`,
);
process.stdout.write(`\nbuilt -> ${path.relative(REPO, OUT_DIR)}\n`);
