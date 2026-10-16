#!/usr/bin/env node
// Every gate in the repository, in one command.
//
//   node scripts/verify.mjs            # full: rebuild everything, then test
//   node scripts/verify.mjs --quick    # skip the wasm/extension rebuild
//
// Stages:
//   1. cargo test --workspace                    (library, CLI, language server)
//   2. cargo build --release -p dpm-nbe-lsp      (the binary the extension spawns)
//   3. node web/build.mjs                        (wasm module for the browser)
//   4. node web/tools/smoke.mjs                  (the wasm server, end to end)
//   5. node web/tools/client-smoke.mjs           (the web IDE's LSP client)
//   6. node vscode/scripts/build.mjs --skip-wasm (bundles + worker transport test)
//   7. node vscode/client/test/jsonrpc.test.mjs  (Microsoft's jsonrpc vs the binary)

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const quick = process.argv.includes('--quick');

const steps = [
  { name: 'cargo test (workspace)', command: 'cargo', args: ['test', '--workspace'] },
  { name: 'cargo build --release (server)', command: 'cargo', args: ['build', '--release', '-p', 'dpm-nbe-lsp'] },
];

if (!quick) {
  steps.push({ name: 'build wasm module', command: process.execPath, args: [path.join(REPO, 'web', 'build.mjs')] });
}

steps.push(
  { name: 'web smoke (wasm server)', command: process.execPath, args: [path.join(REPO, 'web', 'tools', 'smoke.mjs')] },
  { name: 'web smoke (LSP client)', command: process.execPath, args: [path.join(REPO, 'web', 'tools', 'client-smoke.mjs')] },
  {
    name: 'vs code extension',
    command: process.execPath,
    args: [path.join(REPO, 'vscode', 'scripts', 'build.mjs'), '--skip-wasm'],
  },
  {
    name: 'jsonrpc interop (binary)',
    command: process.execPath,
    args: [path.join(REPO, 'vscode', 'client', 'test', 'jsonrpc.test.mjs')],
  },
);

const results = [];
for (const step of steps) {
  process.stdout.write(`\n${'='.repeat(72)}\n${step.name}\n${'='.repeat(72)}\n`);
  const started = performance.now();
  const result = spawnSync(step.command, step.args, {
    cwd: REPO,
    stdio: 'inherit',
    // No shell: every command here is a real executable, and skipping the shell
    // keeps the arguments exactly as written.
    shell: false,
  });
  const seconds = ((performance.now() - started) / 1000).toFixed(1);
  const ok = !result.error && result.status === 0;
  results.push({ name: step.name, ok, seconds, note: result.error?.message ?? '' });
}

process.stdout.write(`\n${'='.repeat(72)}\nsummary\n${'='.repeat(72)}\n`);
for (const result of results) {
  process.stdout.write(
    `${result.ok ? 'PASS' : 'FAIL'}  ${result.seconds.padStart(6)}s  ${result.name}${result.note ? `  (${result.note})` : ''}\n`,
  );
}
const failed = results.filter((r) => !r.ok);
if (failed.length > 0) {
  process.stdout.write(`\n${failed.length} of ${results.length} stages failed\n`);
  process.exit(1);
}
process.stdout.write(`\nall ${results.length} stages passed\n`);
