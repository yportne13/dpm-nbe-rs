#!/usr/bin/env node
// Drives `client/src/transport.js` — the code the extension's Web Worker runs —
// in Node, against the real wasm module, with an in-memory message channel.
//
//   node client/test/transport.test.mjs
//
// Exit codes: 0 = pass, 1 = a check failed, 2 = the module is not built yet.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { startTransport } from '../src/transport.js';

const CLIENT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const VSCODE = path.dirname(CLIENT);
const REPO = path.dirname(VSCODE);

// Prefer the copies the extension actually ships; fall back to the web build.
const CANDIDATES = [
  path.join(VSCODE, 'media'),
  path.join(REPO, 'web', 'www', 'pkg', 'dpm-nbe-wasm'),
];

let failures = 0;
let checks = 0;
function ok(label, condition, detail = '') {
  checks += 1;
  if (condition) {
    process.stdout.write(`  \u2713 ${label}\n`);
  } else {
    failures += 1;
    process.stdout.write(`  \u2717 ${label}${detail ? `  ${detail}` : ''}\n`);
  }
}

let glueUrl = null;
let wasmPath = null;
for (const dir of CANDIDATES) {
  try {
    await readFile(path.join(dir, 'dpm_nbe_wasm_bg.wasm'));
    glueUrl = pathToFileURL(path.join(dir, 'dpm_nbe_wasm.js')).href;
    wasmPath = path.join(dir, 'dpm_nbe_wasm_bg.wasm');
    break;
  } catch {
    /* try the next candidate */
  }
}
if (!glueUrl) {
  process.stdout.write('the wasm module is not built\nbuild it first:  node web/build.mjs\n');
  process.exit(2);
}

const outbox = [];
let handler = null;
const received = [];

const transport = await startTransport({
  loadWasm: async () => {
    const module = await import(glueUrl);
    await module.default({ module_or_path: await readFile(wasmPath) });
    return module;
  },
  postMessage: (message) => outbox.push(message),
  subscribe: (callback) => {
    handler = callback;
    return () => {
      handler = null;
    };
  },
  onLog: (direction, message) => received.push({ direction, message }),
});

ok('startTransport initialises a session', typeof transport.session.send === 'function');
ok('startTransport subscribes to incoming messages', typeof handler === 'function');

/** Ask the transport one question, like the worker's message event does. */
function ask(message) {
  outbox.length = 0;
  handler(message);
  return outbox.slice();
}

const initialize = ask({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: null } });
ok('initialize is answered', initialize.length === 1 && initialize[0].result?.capabilities, JSON.stringify(initialize));
ok('the answer is a JSON-RPC response', initialize[0].jsonrpc === '2.0' && initialize[0].id === 1);
ok('the server identifies itself', initialize[0].result.serverInfo.name === 'dpm-nbe-lsp');

const opened = ask({
  jsonrpc: '2.0',
  method: 'textDocument/didOpen',
  params: {
    textDocument: {
      uri: 'file:///test.dpm',
      languageId: 'dpm-nbe',
      version: 1,
      text: 'let bad : Nat = U; U',
    },
  },
});
ok('didOpen pushes diagnostics through the transport', opened.length === 1 && opened[0].method === 'textDocument/publishDiagnostics');
ok('the diagnostic is the checker error', /type mismatch/.test(opened[0].params.diagnostics[0].message));

const hover = ask({
  jsonrpc: '2.0',
  id: 2,
  method: 'textDocument/hover',
  params: { textDocument: { uri: 'file:///test.dpm' }, text: 'plus zero zero', position: { line: 0, character: 1 } },
});
ok('hover is answered', /primitive addition/.test(hover[0]?.result?.contents?.value ?? ''), JSON.stringify(hover));
ok('requests keep their ids', hover[0].id === 2);

const evaluate = ask({
  jsonrpc: '2.0',
  id: 3,
  method: 'dpm-nbe/evaluate',
  params: { text: 'plus (suc zero) zero', mode: 'nf' },
});
ok('the extension request works', evaluate[0].result.ok === true && evaluate[0].result.output.startsWith('succ zero'));

// A malformed frame must not take the transport down.
const invalid = ask('not a message');
ok('a non-object message gets a JSON-RPC invalid-request error', invalid[0]?.error?.code === -32600, JSON.stringify(invalid));
const unknown = ask({ jsonrpc: '2.0', id: 4, method: 'textDocument/typeDefinition', params: {} });
ok('an unknown method gets method-not-found', unknown[0]?.error?.code === -32601);
ok('the session still works afterwards', ask({ jsonrpc: '2.0', id: 5, method: 'initialize', params: {} })[0].result?.capabilities !== undefined);

ok('the log saw both directions', received.some((e) => e.direction === 'receive') && received.some((e) => e.direction === 'send'));

transport.dispose();
ok('dispose unsubscribes', handler === null);

process.stdout.write(`\n${checks - failures}/${checks} checks passed\n`);
if (failures > 0) {
  process.stdout.write(`TRANSPORT TEST FAIL (${failures} failing)\n`);
  process.exit(1);
}
process.stdout.write('TRANSPORT TEST PASS\n');
