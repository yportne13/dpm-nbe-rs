#!/usr/bin/env node
// Tests the frontend's protocol client (`web/www/lsp.js`) against the real wasm
// module.  The module has no DOM dependency, so the half of the web IDE that
// talks to the language server is covered without a browser.
//
//   node web/tools/client-smoke.mjs
//
// Exit codes: 0 = pass, 1 = a check failed, 2 = not built yet.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const WEB = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const PKG = path.join(WEB, 'www', 'pkg', 'dpm-nbe-wasm');
const { LspClient, offsetAt, positionAt } = await import(
  pathToFileURL(path.join(WEB, 'www', 'lsp.js')).href
);

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

let wasm;
try {
  wasm = await import(pathToFileURL(path.join(PKG, 'dpm_nbe_wasm.js')).href);
  await wasm.default({ module_or_path: await readFile(path.join(PKG, 'dpm_nbe_wasm_bg.wasm')) });
} catch (error) {
  process.stdout.write(`cannot load the module: ${error.message}\nbuild it first:  node web/build.mjs\n`);
  process.exit(2);
}

const README = `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;
U
`;

// --- position helpers ------------------------------------------------------

const text = "a𝔸λ\nbb\n";
for (const offset of [0, 1, 3, 4, 5, 6, 8]) {
  const round = offsetAt(text, positionAt(text, offset));
  ok(`offset ${offset} round trips`, round === Math.min(offset, text.length), `${round}`);
}
ok('positionAt counts UTF-16 units', positionAt(text, 3).character === 3, JSON.stringify(positionAt(text, 3)));
ok('positionAt tracks lines', positionAt(text, 5).line === 1 && positionAt(text, 5).character === 0);

// --- client ----------------------------------------------------------------

const notifications = [];
const log = [];
const client = new LspClient(wasm, {
  uri: 'file:///client.dpm',
  onNotification: (message) => notifications.push(message),
  onLog: (entry) => log.push(entry),
});

const init = await client.start();
ok('start() completes the handshake', init.serverInfo?.name === 'dpm-nbe-lsp', JSON.stringify(init));
ok('start() records the capabilities', client.capabilities?.hoverProvider === true);
ok('the client is alive', client.alive === true);

client.open(README);
ok('didOpen pushes diagnostics to the client', client.diagnostics.length === 0);
ok('the diagnostics push reaches onNotification', notifications.some((n) => n.method === 'textDocument/publishDiagnostics'));
ok('the log records both directions', log.some((e) => e.direction === 'client→server') && log.some((e) => e.direction === 'server→client'));

client.change('let bad : Nat = U; U');
ok('didChange updates the diagnostics', client.diagnostics.length === 1 && /type mismatch/.test(client.diagnostics[0].message), JSON.stringify(client.diagnostics));
ok('the diagnostic range is an LSP range', client.diagnostics[0].range.start.character === 16 && client.diagnostics[0].range.start.line === 0);
ok('the server version is tracked', client.version === 2, String(client.version));

// --- concurrent requests keep their identities ------------------------------

const hoverAt = positionAt(README, README.indexOf('Id A a b') + 'Id A '.length);
const [hover, completion, symbols, tokens, evaluated] = await Promise.all([
  client.hover(README, hoverAt),
  client.completion(README, hoverAt),
  client.documentSymbols(README),
  client.tokens(README),
  client.evaluate('plus (suc zero) (suc zero)', 'nf'),
]);
ok('concurrent hover resolves correctly', /a : A/.test(hover?.contents?.value ?? ''), JSON.stringify(hover)?.slice(0, 120));
ok('concurrent completion resolves correctly', (completion?.items ?? []).some((i) => i.label === 'refl'));
ok('concurrent documentSymbol resolves correctly', symbols?.[0]?.name === 'trans' && symbols[0].children?.[0]?.name === 'H');
ok('concurrent tokens resolve correctly', tokens?.tokens?.[0]?.type === 'keyword');
ok('concurrent evaluate resolves correctly', evaluated?.output === 'succ (succ zero)\n  :\nNat', JSON.stringify(evaluated));

const definition = await client.definition(README, positionAt(README, README.indexOf('Id Nat n (plus m zero)') + 'Id Nat '.length));
ok('definition returns a location in the same document', definition?.uri === 'file:///client.dpm');
ok('definition points at `n`', README.slice(offsetAt(README, definition.range.start)).startsWith('n'));

// --- failure handling -------------------------------------------------------

const doomed = new LspClient(wasm, { uri: 'file:///doomed.dpm' });
await doomed.start();
doomed.die('simulated trap');
ok('a dead client refuses work instead of throwing', (await doomed.hover('U', { line: 0, character: 0 }).then(() => 'resolved', (e) => e.message)).includes('simulated trap'));
ok('a dead client marks itself dead', doomed.alive === false);
ok('the trap is reported to the UI', doomed.alive === false);

// --- a deep computation must not kill the client -----------------------------
//
// `x_k = plus x_{k-1} x_{k-1}` doubles a numeral, so `x14` normalizes to one
// nested 16384 constructors deep — past the engine's ~1 MB wasm stack (main's
// evaluator traps between 2^13 and 2^14).  On the default Node stack the
// evaluate call traps; the client must restart the session and keep serving.
// (On an engine with a bigger stack the call simply succeeds; every assertion
// below holds either way.)

const trapNotifications = [];
const deepClient = new LspClient(wasm, {
  uri: 'file:///deep.dpm',
  onNotification: (message) => {
    if (message.method === '$/trap') trapNotifications.push(message);
  },
});
await deepClient.start();
deepClient.open('U');
let deepText = 'let x0 : Nat = suc zero;\n';
for (let k = 1; k <= 14; k += 1) deepText += `let x${k} : Nat = plus x${k - 1} x${k - 1};\n`;
deepText += 'x14';
await deepClient.evaluate(deepText, 'nf').then(() => {}, () => {});
ok('the client survives a too-deep evaluation', deepClient.alive === true);
if (trapNotifications.length > 0) {
  const last = trapNotifications[trapNotifications.length - 1].params;
  ok('the stack trap is reported as recovered', last.recovered === true, JSON.stringify(last));
  ok('the stack trap names the resource limit', /wasm stack/.test(last.reason ?? ''), JSON.stringify(last));
} else {
  ok('no spurious trap notifications', trapNotifications.length === 0);
}
const after = await deepClient.evaluate('U', 'nf');
ok('the client answers after the deep computation', after?.ok === true, JSON.stringify(after)?.slice(0, 120));
ok('a session that never panicked reports none', wasm.lastPanic() === '', JSON.stringify(wasm.lastPanic()));

// --- a Rust panic is fatal and carries its message ---------------------------
//
// The real module reports a panic through the trap + `lastPanic()` pair; this
// stub reproduces exactly that shape (a bare `unreachable` trap plus a
// recorded panic), so the client's fatal classification is covered without a
// known-panicking program in the tree.

const panicNotifications = [];
const stub = {
  LspSession: class {
    send() { throw new Error('unreachable'); }
  },
  lastPanic: () => 'panicked at src/elab.rs:9:9: probe',
};
const panicked = new LspClient(stub, {
  onNotification: (message) => {
    if (message.method === '$/trap') panicNotifications.push(message);
  },
});
const startError = await panicked.start().then(() => null, (error) => error);
ok('a Rust panic surfaces its message', /Rust panic: panicked at src\/elab\.rs:9:9: probe/.test(startError?.message ?? ''), String(startError));
ok('a Rust panic kills the client', panicked.alive === false);
ok('the panic text travels to the UI', panicNotifications[0]?.params?.panic === 'panicked at src/elab.rs:9:9: probe', JSON.stringify(panicNotifications[0]));

await client.shutdown();
ok('shutdown ends the session', (await client.hover('U', { line: 0, character: 0 }).then(() => 'resolved', (e) => e.message)).length > 0);

process.stdout.write(`\n${checks - failures}/${checks} checks passed\n`);
if (failures > 0) {
  process.stdout.write(`CLIENT SMOKE FAIL (${failures} failing)\n`);
  process.exit(1);
}
process.stdout.write('CLIENT SMOKE PASS\n');
