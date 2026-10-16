#!/usr/bin/env node
// Integration smoke test for the browser language server.
//
//   node web/tools/smoke.mjs
//
// Loads the built module in Node with the wasm bytes read from disk (so no
// browser and no HTTP MIME handling is involved), then drives the *same*
// JSON-RPC conversation the web IDE performs: initialize, didOpen, hover,
// completion, definition, documentSymbol, evaluate, tokens — and checks that a
// pathological input comes back as a diagnostic instead of a trap.
//
// Exit codes: 0 = pass, 1 = a check failed, 2 = not built yet.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const WEB = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const PKG = path.join(WEB, 'www', 'pkg', 'dpm-nbe-wasm');
const GLUE = path.join(PKG, 'dpm_nbe_wasm.js');
const WASM = path.join(PKG, 'dpm_nbe_wasm_bg.wasm');

const README = `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;
let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);
U
`;

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

/** Byte offset -> 0-based LSP position (UTF-16 code units per line). */
function positionOf(text, offset) {
  const before = text.slice(0, offset);
  const line = before.split('\n').length - 1;
  const lineStart = before.lastIndexOf('\n') + 1;
  return { line, character: text.slice(lineStart, offset).length };
}

function offsetOf(text, position) {
  const lines = text.split('\n');
  let offset = 0;
  for (let i = 0; i < position.line; i += 1) {
    offset += lines[i].length + 1;
  }
  return offset + position.character;
}

function offsetOfNeedle(text, needle, extra = 0) {
  const at = text.indexOf(needle);
  if (at < 0) throw new Error(`needle ${JSON.stringify(needle)} not found`);
  return at + extra;
}

function main() {
  return import(pathToFileURL(GLUE).href);
}

let mod;
try {
  mod = await main();
} catch (error) {
  process.stdout.write(`cannot load ${GLUE}\n  ${error.message}\n`);
  process.stdout.write('build it first:  node web/build.mjs\n');
  process.exit(2);
}

const bytes = await readFile(WASM);
await mod.default({ module_or_path: bytes });

/** Send one JSON-RPC message, return the parsed array of replies. */
function send(session, message) {
  return JSON.parse(session.send(JSON.stringify(message)));
}

function request(id, method, params) {
  return { jsonrpc: '2.0', id, method, params };
}

function note(method, params) {
  return { jsonrpc: '2.0', method, params };
}

const info = JSON.parse(mod.info());
ok('info() names the server', info.name === 'dpm-nbe-lsp' && /^\d+\.\d+/.test(info.version), JSON.stringify(info));
ok('info() advertises the LSP surface', info.methods.includes('textDocument/hover') && info.methods.includes('dpm-nbe/evaluate'));
ok('info() declares the .dpm language id', info.language.id === 'dpm-nbe' && info.language.extensions.includes('.dpm'));

// --- handshake ------------------------------------------------------------

const session = new mod.LspSession();
let replies = send(session, request(1, 'initialize', { processId: null, capabilities: {} }));
ok('initialize returns exactly one reply', replies.length === 1);
const caps = replies[0].result.capabilities;
ok('capabilities advertise hover/definition/symbols', caps.hoverProvider === true && caps.definitionProvider === true && caps.documentSymbolProvider === true);
ok('capabilities advertise full sync + openClose', caps.textDocumentSync.change === 1 && caps.textDocumentSync.openClose === true);
ok('session reports itself initialized', session.initialized === true);

// --- diagnostics ----------------------------------------------------------

const uri = 'file:///readme.dpm';
replies = send(session, note('textDocument/didOpen', {
  textDocument: { uri, languageId: 'dpm-nbe', version: 1, text: README },
}));
ok('didOpen pushes diagnostics', replies.length === 1 && replies[0].method === 'textDocument/publishDiagnostics');
ok('a correct program has no diagnostics', replies[0].params.diagnostics.length === 0);

const badUri = 'file:///bad.dpm';
replies = send(session, note('textDocument/didOpen', {
  textDocument: { uri: badUri, languageId: 'dpm-nbe', version: 1, text: 'let bad : Nat = U; U' },
}));
const bad = replies[0].params.diagnostics;
ok('a type error is reported', bad.length === 1 && bad[0].severity === 1 && bad[0].code === 'type-error');
ok('the diagnostic carries the message', /type mismatch/.test(bad[0].message), bad[0]?.message);
ok('the diagnostic points at column 16', bad[0].range.start.line === 0 && bad[0].range.start.character === 16, JSON.stringify(bad[0]?.range));
ok('the diagnostic carries the rendered caret diagram', /^\s*\|\s*\^/m.test(bad[0].data?.rendered ?? ''));

// A ranged edit (defensive: the server advertises full sync) must be applied.
// `let bad : Nat = U; U` -> `let bad : Nat = zero; U` is then well typed.
replies = send(session, note('textDocument/didChange', {
  textDocument: { uri: badUri, version: 2 },
  contentChanges: [{
    range: { start: { line: 0, character: 16 }, end: { line: 0, character: 17 } },
    text: 'zero',
  }],
}));
ok('a ranged didChange clears the diagnostic', replies[0].params.diagnostics.length === 0 && replies[0].params.version === 2, JSON.stringify(replies[0].params.diagnostics));

// --- hover ----------------------------------------------------------------

const hoverAt = positionOf(README, offsetOfNeedle(README, 'Id A a b', 'Id A '.length));
replies = send(session, request(2, 'textDocument/hover', { textDocument: { uri }, position: hoverAt }));
const hoverText = replies[0].result?.contents?.value ?? '';
ok('hover on a binder reports its type', /a : A/.test(hoverText), hoverText);
ok('hover reports the binding form', /function-type binder/.test(hoverText));

const plusAt = positionOf(README, offsetOfNeedle(README, 'plus m zero'));
replies = send(session, request(3, 'textDocument/hover', { textDocument: { uri }, position: plusAt }));
ok('hover documents a keyword', /primitive addition/.test(replies[0].result?.contents?.value ?? ''));

// --- completion -----------------------------------------------------------

const scopeText = 'let x : Nat = zero;\n';
replies = send(session, request(4, 'textDocument/completion', {
  textDocument: { uri: 'file:///completion.dpm' },
  text: scopeText,
  position: positionOf(scopeText, scopeText.length),
}));
const labels = replies[0].result.items.map((i) => i.label);
ok('completion offers names in scope', labels.includes('x'), labels.join(','));
ok('completion offers the vocabulary', labels.includes('Nat') && labels.includes('plus') && labels.includes('(!)'));
const xItem = replies[0].result.items.find((i) => i.label === 'x');
ok('a completed name carries its type', xItem.kind === 6 && xItem.detail === 'Nat', JSON.stringify(xItem));

// --- definition -----------------------------------------------------------

const useAt = positionOf(README, offsetOfNeedle(README, 'Id Nat n (plus m zero)', 'Id Nat '.length));
replies = send(session, request(5, 'textDocument/definition', { textDocument: { uri }, position: useAt }));
const defRange = replies[0].result?.range;
ok('definition finds the binder', defRange !== undefined && defRange.start.line === 2, JSON.stringify(defRange));
if (defRange) {
  const at = offsetOf(README, defRange.start);
  ok('definition points at the identifier `n`', README.slice(at, at + 1) === 'n', JSON.stringify(README.slice(at, at + 4)));
}
replies = send(session, request(6, 'textDocument/definition', {
  textDocument: { uri }, position: positionOf(README, offsetOfNeedle(README, 'Nat (suc zero)')),
}));
ok('a keyword has no definition', replies[0].result === null);

// --- document symbols -----------------------------------------------------

replies = send(session, request(7, 'textDocument/documentSymbol', { textDocument: { uri } }));
const symbols = replies[0].result;
ok('document symbols list the definitions', symbols.length === 1 && symbols[0].name === 'trans');
ok('symbols nest the way the syntax does', symbols[0].children?.[0]?.name === 'H' && symbols[0].children[0].children?.[0]?.name === 'C');
ok('a symbol carries its type as detail', /^\(A : U\)/.test(symbols[0].detail ?? ''), symbols[0].detail);

// --- evaluation -----------------------------------------------------------

replies = send(session, request(8, 'dpm-nbe/evaluate', {
  textDocument: { uri: 'file:///run.dpm' },
  text: 'plus (suc zero) (suc zero)',
  mode: 'nf',
}));
ok('evaluate returns the normal form and the type', replies[0].result.ok === true && replies[0].result.output === 'succ (succ zero)\n  :\nNat', replies[0].result.output);

replies = send(session, request(9, 'dpm-nbe/evaluate', { text: 'refl', mode: 'nf' }));
ok('evaluate reports language errors as data', replies[0].result.ok === false && /Can't infer type for refl/.test(replies[0].result.error));
ok('evaluate reports where it failed', replies[0].result.range?.start?.line === 0);

const typed = JSON.parse(mod.check('(A : U) -> A -> A', 'type'));
ok('check() is a shortcut for `dpm-nbe type`', typed.ok === true && typed.output === 'U', JSON.stringify(typed));

// --- tokens ---------------------------------------------------------------

replies = send(session, request(10, 'dpm-nbe/tokens', { text: '\u03bb x . x -- done' }));
const tokens = replies[0].result.tokens;
const tokenSource = '\u03bb x . x -- done';
ok('tokens start with the lambda keyword', tokens[0].type === 'keyword' && tokens[0].range.start.character === 0);
ok('tokens classify identifiers and comments', tokens[1].type === 'ident' && tokens[tokens.length - 1].type === 'comment', JSON.stringify(tokens));
// The non-whitespace tokens must tile the line without gaps or overlaps.
const covered = tokens.map((t) => [t.range.start.character, t.range.end.character]);
let contiguous = covered.length > 0 && covered[0][0] === 0;
for (let i = 1; i < covered.length; i += 1) {
  if (covered[i][0] < covered[i - 1][1]) contiguous = false;
}
ok('token ranges tile the line', contiguous && covered[covered.length - 1][1] === tokenSource.length, JSON.stringify(covered));

// --- robustness -----------------------------------------------------------

const deepNat = `let x : Nat = ${'suc ('.repeat(150)}zero${')'.repeat(150)}; x`;
const deep = JSON.parse(mod.check(deepNat, 'nf'));
ok('150 nested constructors do not overflow the shadow stack', deep.ok === true, JSON.stringify(deep).slice(0, 200));
ok('... and normalize to the right numeral', deep.ok && deep.output.startsWith('succ (succ'), deep.output?.slice(0, 40));

const tooDeep = `${'('.repeat(2000)}U`;
replies = send(session, request(11, 'dpm-nbe/diagnostics', { text: tooDeep }));
ok('2000 nested parentheses come back as a diagnostic', replies[0].result.length === 1, JSON.stringify(replies[0].result));
ok('the server is still alive afterwards', send(session, request(12, 'shutdown', null))[0].result === null);

// --- shutdown -------------------------------------------------------------

const bye = send(session, request(13, 'textDocument/hover', { text: 'U', position: { line: 0, character: 0 } }));
ok('requests still work after 2000-deep input', /universe/.test(bye[0].result?.contents?.value ?? ''));

process.stdout.write(`\n${checks - failures}/${checks} checks passed\n`);
if (failures > 0) {
  process.stdout.write(`SMOKE FAIL (${failures} failing)\n`);
  process.exit(1);
}
process.stdout.write('SMOKE PASS\n');
