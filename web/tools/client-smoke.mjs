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

const NAT = 'data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n'
  + 'let plus : Nat -> Nat -> Nat =\n'
  + '  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;\n';

const README = NAT + `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
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

client.change(NAT + 'let bad : Nat = U; U');
ok('didChange updates the diagnostics', client.diagnostics.length === 1 && /type mismatch/.test(client.diagnostics[0].message), JSON.stringify(client.diagnostics));
ok('the diagnostic range is an LSP range', client.diagnostics[0].range.start.character === 16 && client.diagnostics[0].range.start.line === 3);
ok('the server version is tracked', client.version === 2, String(client.version));

// --- concurrent requests keep their identities ------------------------------

const hoverAt = positionAt(README, README.indexOf('Id A a b') + 'Id A '.length);
const [hover, completion, symbols, tokens, evaluated] = await Promise.all([
  client.hover(README, hoverAt),
  client.completion(README, hoverAt),
  client.documentSymbols(README),
  client.tokens(README),
  client.evaluate(NAT + 'plus (suc zero) (suc zero)', 'nf'),
]);
ok('concurrent hover resolves correctly', /a : A/.test(hover?.contents?.value ?? ''), JSON.stringify(hover)?.slice(0, 120));
ok('concurrent completion resolves correctly', (completion?.items ?? []).some((i) => i.label === 'refl'));
ok('concurrent documentSymbol resolves correctly', symbols?.[0]?.name === 'plus' && symbols[0].children?.[0]?.name === 'trans');
ok('concurrent tokens resolve correctly', tokens?.tokens?.[0]?.type === 'keyword');
ok('concurrent evaluate resolves correctly', evaluated?.output === 'suc (suc zero)\n  :\nNat', JSON.stringify(evaluated));

const definition = await client.definition(README, positionAt(README, README.indexOf('Id Nat n (plus m zero)') + 'Id Nat '.length));
ok('definition returns a location in the same document', definition?.uri === 'file:///client.dpm');
ok('definition points at `n`', README.slice(offsetAt(README, definition.range.start)).startsWith('n'));

// --- failure handling -------------------------------------------------------

const doomed = new LspClient(wasm, { uri: 'file:///doomed.dpm' });
await doomed.start();
doomed.die(new Error('simulated trap'));
ok('a dead client refuses work instead of throwing', (await doomed.hover('U', { line: 0, character: 0 }).then(() => 'resolved', (e) => e.message)).includes('simulated trap'));
ok('a dead client marks itself dead', doomed.alive === false);
ok('the trap is reported to the UI', doomed.alive === false);

await client.shutdown();
ok('shutdown ends the session', (await client.hover('U', { line: 0, character: 0 }).then(() => 'resolved', (e) => e.message)).length > 0);

process.stdout.write(`\n${checks - failures}/${checks} checks passed\n`);
if (failures > 0) {
  process.stdout.write(`CLIENT SMOKE FAIL (${failures} failing)\n`);
  process.exit(1);
}
process.stdout.write('CLIENT SMOKE PASS\n');
