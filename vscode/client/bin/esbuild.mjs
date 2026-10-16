#!/usr/bin/env node
// Bundle the three extension artifacts with esbuild.
//
//   node client/bin/esbuild.mjs            # one shot
//   node client/bin/esbuild.mjs --watch    # rebuild on change
//
// * dist/desktop/extension.js — CommonJS, `vscode` left external (the desktop
//   extension host provides it).
// * dist/web/extension.js     — CommonJS, loaded by the browser extension host.
// * dist/web/worker.js        — an ES module Web Worker (it uses `import()`),
//   so it is *not* CommonJS like the extension entry points.

import { build, context } from 'esbuild';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CLIENT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const watch = process.argv.includes('--watch');

const common = {
  bundle: true,
  sourcemap: true,
  logLevel: 'info',
  target: 'es2022',
  external: ['vscode'],
  loader: { '.wasm': 'file' },
};

const targets = [
  {
    ...common,
    entryPoints: [path.join(CLIENT, 'src', 'extension.desktop.js')],
    outfile: path.join(CLIENT, 'dist', 'desktop', 'extension.js'),
    platform: 'node',
    format: 'cjs',
  },
  {
    ...common,
    entryPoints: [path.join(CLIENT, 'src', 'extension.web.js')],
    outfile: path.join(CLIENT, 'dist', 'web', 'extension.js'),
    platform: 'browser',
    format: 'cjs',
  },
  {
    ...common,
    entryPoints: [path.join(CLIENT, 'src', 'worker.js')],
    outfile: path.join(CLIENT, 'dist', 'web', 'worker.js'),
    platform: 'browser',
    format: 'esm',
  },
];

if (watch) {
  const contexts = await Promise.all(targets.map((options) => context(options)));
  await Promise.all(contexts.map((ctx) => ctx.watch()));
  console.log('esbuild: watching');
} else {
  await Promise.all(targets.map((options) => build(options)));
  console.log('esbuild: bundled desktop, web and worker');
}
