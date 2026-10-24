#!/usr/bin/env node
// Talks to the *native* `dpm-nbe-lsp` binary with Microsoft's own
// `vscode-jsonrpc` implementation, i.e. not with our own framing code and not
// with our own client.
//
//   node client/test/jsonrpc.test.mjs
//
// This is the interoperability check: if the server's `Content-Length` framing,
// its JSON-RPC shape, its notification ordering or its shutdown sequence were
// off, a standard client would notice.
//
// Exit codes: 0 = pass, 1 = a check failed, 2 = the binary is not built yet.

import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
// `vscode-jsonrpc` has no `exports` map, so the Node entry needs its extension
// spelled out to be importable as ESM.
import {
  createMessageConnection,
  StreamMessageReader,
  StreamMessageWriter,
} from 'vscode-jsonrpc/node.js';

const CLIENT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const VSCODE = path.dirname(CLIENT);
const REPO = path.dirname(VSCODE);
const BINARY = process.platform === 'win32' ? 'dpm-nbe-lsp.exe' : 'dpm-nbe-lsp';

const CANDIDATES = [
  path.join(REPO, 'target', 'release', BINARY),
  path.join(REPO, 'target', 'debug', BINARY),
  path.join(REPO, 'lsp', 'target', 'release', BINARY),
  path.join(REPO, 'lsp', 'target', 'debug', BINARY),
];
const server = CANDIDATES.find((candidate) => existsSync(candidate));

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

if (!server) {
  process.stdout.write(
    `the language server binary is not built (looked in ${CANDIDATES.join(', ')})\n` +
      'build it first:  cargo build --release -p dpm-nbe-lsp\n',
  );
  process.exit(2);
}

const README = `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;
U
`;

const child = spawn(server, [], { stdio: ['pipe', 'pipe', 'inherit'] });
const connection = createMessageConnection(
  new StreamMessageReader(child.stdout),
  new StreamMessageWriter(child.stdin),
);
connection.listen();

const notifications = [];
connection.onNotification((method, params) => {
  notifications.push({ method, params });
});
connection.onUnhandledNotification((message) => {
  notifications.push({ method: message.method, params: message.params, unhandled: true });
});

const exited = new Promise((resolve) => {
  child.on('exit', (code) => resolve(code));
});

const uri = 'file:///jsonrpc.dpm';

const init = await connection.sendRequest('initialize', {
  processId: process.pid,
  clientInfo: { name: 'vscode-jsonrpc test' },
  capabilities: {},
});
ok('a standard client completes the handshake', init?.serverInfo?.name === 'dpm-nbe-lsp', JSON.stringify(init?.serverInfo));
ok('the handshake carries the capabilities', init?.capabilities?.textDocumentSync?.openClose === true);
connection.sendNotification('initialized', {});

connection.sendNotification('textDocument/didOpen', {
  textDocument: { uri, languageId: 'dpm-nbe', version: 1, text: README },
});
await new Promise((resolve) => setTimeout(resolve, 400));
const clean = notifications.find((n) => n.method === 'textDocument/publishDiagnostics');
ok('a correct program raises no diagnostics', clean !== undefined && clean.params.diagnostics.length === 0, JSON.stringify(clean));

connection.sendNotification('textDocument/didChange', {
  textDocument: { uri, version: 2 },
  contentChanges: [{ text: 'data Nat : U where zero : Nat; let bad : Nat = U; U' }],
});
await new Promise((resolve) => setTimeout(resolve, 400));
const dirty = notifications
  .filter((n) => n.method === 'textDocument/publishDiagnostics')
  .at(-1);
ok('a type error is published as a notification', dirty?.params?.diagnostics?.length === 1, JSON.stringify(dirty));
ok('the diagnostic position matches the CLI', dirty?.params?.diagnostics?.[0]?.range?.start?.character === 47);
ok('the diagnostic names its source', dirty?.params?.diagnostics?.[0]?.source === 'dpm-nbe');
ok('the diagnostic carries the rendered caret diagram', /\^/.test(dirty?.params?.diagnostics?.[0]?.data?.rendered ?? ''));

const hover = await connection.sendRequest('textDocument/hover', {
  textDocument: { uri },
  text: 'let x : Nat = zero; x',
  position: { line: 0, character: 21 },
});
ok('hover answers with markdown', /x : Nat/.test(hover?.contents?.value ?? ''), JSON.stringify(hover));

// The body of `let y : ...; ` starts right after the semicolon, so that is
// where `y` comes into scope — completing *on* the semicolon must not offer it.
const completion = await connection.sendRequest('textDocument/completion', {
  textDocument: { uri },
  text: 'let y : Nat = zero; y',
  position: { line: 0, character: 20 },
});
ok('completion answers with an item list', (completion?.items ?? []).some((i) => i.label === 'y'), JSON.stringify(completion?.items?.map((i) => i.label)));
ok('completion items are labelled with an LSP kind', completion.items.every((i) => typeof i.kind === 'number'));

const symbols = await connection.sendRequest('textDocument/documentSymbol', {
  textDocument: { uri },
  text: README,
});
ok('document symbols answer with the definitions', symbols?.[0]?.name === 'trans' && symbols[0].selectionRange !== undefined);

const evaluated = await connection.sendRequest('dpm-nbe/evaluate', {
  text: 'data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; suc (suc zero)',
  mode: 'nf',
});
ok('the server extension request works from a standard client', evaluated?.output === 'suc (suc zero)\n  :\nNat', JSON.stringify(evaluated));

const unknown = await connection
  .sendRequest('textDocument/typeDefinition', { textDocument: { uri }, position: { line: 0, character: 0 } })
  .then(() => null, (error) => error);
ok('an unimplemented request becomes a JSON-RPC error', unknown?.code === -32601, JSON.stringify(unknown?.message ?? unknown));

await connection.sendRequest('shutdown');
connection.sendNotification('exit');
connection.dispose();

const code = await Promise.race([
  exited,
  new Promise((resolve) => setTimeout(() => resolve('timeout'), 5000)),
]);
ok('the server exits cleanly after exit', code === 0, String(code));

process.stdout.write(`\n${checks - failures}/${checks} checks passed\n`);
if (failures > 0) {
  process.stdout.write(`JSON-RPC TEST FAIL (${failures} failing)\n`);
  process.exit(1);
}
process.stdout.write('JSON-RPC TEST PASS\n');
