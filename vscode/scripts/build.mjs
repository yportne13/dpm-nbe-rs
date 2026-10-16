#!/usr/bin/env node
// Build the whole VS Code extension, in order:
//
//   node vscode/scripts/build.mjs
//
//   1. build the wasm language server   (web/build.mjs)
//   2. copy it next to the extension    (scripts/sync-wasm.mjs)
//   3. install the client dependencies  (npm install, only when missing)
//   4. bundle desktop / web / worker    (client/bin/esbuild.mjs)
//
// Then package with `npx @vscode/vsce package` from `vscode/`.

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const VSCODE = path.dirname(SCRIPT_DIR);
const REPO = path.dirname(VSCODE);
const CLIENT = path.join(VSCODE, 'client');

function run(command, args, cwd = REPO) {
  process.stdout.write(`\n$ ${command} ${args.join(' ')}\n`);
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    cwd,
    // `npm` is a `.cmd` shim on Windows and needs a shell; `node` does not.
    shell: process.platform === 'win32' && command === 'npm',
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

const args = process.argv.slice(2);
if (!args.includes('--skip-wasm')) {
  run(process.execPath, [path.join(REPO, 'web', 'build.mjs')]);
  run(process.execPath, [path.join(SCRIPT_DIR, 'sync-wasm.mjs')]);
}

if (!existsSync(path.join(CLIENT, 'node_modules'))) {
  run('npm', ['install', '--no-audit', '--no-fund'], CLIENT);
}

run('npm', ['run', 'bundle'], CLIENT);
run(process.execPath, [path.join(CLIENT, 'test', 'transport.test.mjs')]);
run(process.execPath, [path.join(CLIENT, 'test', 'jsonrpc.test.mjs')]);

process.stdout.write('\nextension built: dist/desktop, dist/web, media/\n');
process.stdout.write('package it with:  cd vscode && npx @vscode/vsce package\n');
