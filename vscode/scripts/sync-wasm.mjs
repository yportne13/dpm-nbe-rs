#!/usr/bin/env node
// Copy the built wasm language server next to the extension.
//
//   node vscode/scripts/sync-wasm.mjs
//
// Run `node web/build.mjs` first; this keeps one single build of the module so
// the browser IDE and the VS Code web extension cannot drift apart.

import { copyFile, mkdir, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const VSCODE = path.dirname(SCRIPT_DIR);
const REPO = path.dirname(VSCODE);
const FROM = path.join(REPO, 'web', 'www', 'pkg', 'dpm-nbe-wasm');
const TO = path.join(VSCODE, 'media');

const FILES = ['dpm_nbe_wasm.js', 'dpm_nbe_wasm_bg.wasm'];

await mkdir(TO, { recursive: true });
for (const file of FILES) {
  try {
    await stat(path.join(FROM, file));
  } catch {
    console.error(`missing ${path.join(FROM, file)}\nbuild it first:  node web/build.mjs`);
    process.exit(2);
  }
  await copyFile(path.join(FROM, file), path.join(TO, file));
  console.log(`copied ${file}`);
}
// The glue is an ES module; without this marker Node guesses (and warns) when a
// test imports it straight from this directory.  The extension manifest itself
// must stay CommonJS, hence the marker living here.
await writeFile(path.join(TO, 'package.json'), `${JSON.stringify({ type: 'module' }, null, 2)}\n`);
console.log(`wasm module synced -> ${path.relative(REPO, TO)}`);
