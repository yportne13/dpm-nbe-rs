// Wires the editor to the in-browser language server.
//
// Every language feature in this page comes from a real LSP conversation:
// `initialize` at startup, `textDocument/didOpen` + `didChange` on every edit
// (the server answers with `textDocument/publishDiagnostics`), then `hover`,
// `completion`, `definition`, `documentSymbol` and two extensions the server
// adds for this IDE (`dpm-nbe/tokens` for highlighting, `dpm-nbe/evaluate` for
// the "check" panel).  The traffic is visible in the LSP panel.

import { CodeEditor } from './editor.js';
import { LspClient, offsetAt } from './lsp.js';

const PRESETS = {
  readme: {
    label: 'README example (trans / cong / H / C)',
    text: `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;
let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) =
  λ A B f a b refl . refl;
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;
let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);
U
`,
  },
  arithmetic: {
    label: 'Arithmetic (plus normalizes)',
    text: 'plus (suc (suc zero)) (suc (suc (suc zero)))\n',
  },
  types: {
    label: 'A dependent type',
    text: 'let apply : (A : U) (B : A -> U) (f : (x : A) -> B x) (a : A) -> B a =\n  λ A B f a . f a;\n(A : U) -> (B : A -> U) -> ((x : A) -> B x) -> (a : A) -> B a\n',
  },
  absurd: {
    label: 'Absurd pattern (λ A (!))',
    text: 'let C : (A : U) -> Id Nat (suc zero) zero -> A =\n  λ A (!);\nC\n',
  },
  broken: {
    label: 'A type error to look at',
    text: 'let bad : Nat = U; U\n',
  },
  deep: {
    label: 'Deep nesting (150 constructors)',
    text: `let deep : Nat = ${'suc ('.repeat(150)}zero${')'.repeat(150)};\ndeep\n`,
  },
};

const STORAGE_KEY = 'dpm-nbe-web:v1';

const $ = (id) => document.getElementById(id);
const deepLink = new URLSearchParams(location.search).get('preset');
const initialPreset = PRESETS[deepLink] ? deepLink : 'readme';

let wasm = null;
let editor = null;
let client = null;
let hoverTimer = null;
let changeTimer = null;
let completionTimer = null;
let logEntries = [];
let logVisible = false;
let lastResultMode = 'nf';

// ---------------------------------------------------------------------------
// wasm
// ---------------------------------------------------------------------------

function banner(kind, html) {
  const box = $('banner');
  box.className = `banner ${kind}`;
  box.innerHTML = html;
  box.hidden = false;
}

async function loadWasm() {
  const module = await import('./pkg/dpm-nbe-wasm/dpm_nbe_wasm.js');
  await module.default();
  return module;
}

// ---------------------------------------------------------------------------
// panels
// ---------------------------------------------------------------------------

function renderServerStatus() {
  const info = client.serverInfo ?? {};
  const methods = Object.keys(client.capabilities ?? {}).length;
  $('server-status').textContent = `${info.name ?? 'dpm-nbe-lsp'} ${info.version ?? ''}`.trim();
  $('server-detail').textContent = `LSP initialized · ${methods} capabilities`;
}

function renderDiagnostics(diagnostics) {
  editor.setDiagnostics(diagnostics);
  $('diag-count').textContent = String(diagnostics.length);
  const list = $('diagnostics');
  if (diagnostics.length === 0) {
    list.innerHTML = '<li class="empty">no problems</li>';
    return;
  }
  list.innerHTML = diagnostics
    .map((diagnostic) => {
      const line = diagnostic.range.start.line + 1;
      const column = diagnostic.range.start.character + 1;
      const rendered = diagnostic.data?.rendered;
      const severity = (diagnostic.severity ?? 1) === 1 ? 'error' : 'warning';
      return `<li class="diag ${severity}" data-line="${line - 1}" data-character="${diagnostic.range.start.character}">
          <div class="head"><span class="where">${line}:${column}</span>
          <span class="code">${diagnostic.code ?? ''}</span></div>
          <div class="msg">${escapeHtml(diagnostic.message)}</div>
          ${rendered ? `<details><summary>caret diagram</summary><pre>${escapeHtml(rendered)}</pre></details>` : ''}
        </li>`;
    })
    .join('');

  for (const item of list.querySelectorAll('.diag')) {
    item.addEventListener('click', () => {
      editor.setCaretOffset(offsetAt(editor.value, {
        line: Number(item.dataset.line),
        character: Number(item.dataset.character),
      }));
    });
  }
}

function renderSymbols(symbols) {
  const list = $('symbols');
  if (!symbols || symbols.length === 0) {
    list.innerHTML = '<li class="empty">no definitions</li>';
    return;
  }
  const walk = (items, depth) =>
    items
      .map((symbol) => {
        const self = `<li class="symbol" data-line="${symbol.selectionRange.start.line}"
              data-character="${symbol.selectionRange.start.character}"
              style="padding-left:${8 + depth * 14}px">
            <span class="name">${escapeHtml(symbol.name)}</span>
            ${symbol.detail ? `<span class="detail">${escapeHtml(symbol.detail)}</span>` : ''}
          </li>`;
        return self + walk(symbol.children ?? [], depth + 1);
      })
      .join('');
  list.innerHTML = walk(symbols, 0);
  for (const item of list.querySelectorAll('.symbol')) {
    item.addEventListener('click', () => {
      editor.selectRange(
        offsetAt(editor.value, {
          line: Number(item.dataset.line),
          character: Number(item.dataset.character),
        }),
        offsetAt(editor.value, {
          line: Number(item.dataset.line),
          character: Number(item.dataset.character),
        }) + 1,
      );
    });
  }
}

function renderCursor(position) {
  $('cursor').textContent = `line ${position.line + 1}, column ${position.character + 1}`;
}

function renderLog() {
  const body = $('log');
  body.textContent = logEntries
    .slice(-120)
    .map((entry) => {
      const arrow = entry.direction === 'client→server' ? '→' : '←';
      return `${arrow} ${JSON.stringify(entry.message)}`;
    })
    .join('\n');
  body.scrollTop = body.scrollHeight;
}

function escapeHtml(text) {
  return String(text).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]);
}

// ---------------------------------------------------------------------------
// LSP driven features
// ---------------------------------------------------------------------------

async function refresh(text) {
  if (!client?.alive) return;
  const started = performance.now();
  try {
    const [tokens, symbols] = await Promise.all([
      client.tokens(text),
      client.documentSymbols(text),
    ]);
    editor.setTokens(tokens.tokens);
    renderSymbols(symbols);
    $('timing').textContent = `${(performance.now() - started).toFixed(1)} ms / refresh`;
  } catch {
    /* a trap is reported by the client */
  }
}

async function run(mode = lastResultMode) {
  if (!client?.alive) return;
  lastResultMode = mode;
  const started = performance.now();
  try {
    const result = await client.evaluate(editor.value, mode);
    const elapsed = `${(performance.now() - started).toFixed(1)} ms`;
    if (result.ok) {
      $('output').className = 'output ok';
      $('output').textContent = result.output;
      $('output-note').textContent = `checked in ${elapsed}`;
    } else {
      $('output').className = 'output bad';
      $('output').textContent = result.error;
      $('output-note').textContent = `failed in ${elapsed}`;
    }
  } catch {
    /* a trap is reported by the client */
  }
}

async function requestCompletion(request, { automatic }) {
  if (!client?.alive) return;
  if (automatic && request.prefix.length === 0) return;
  try {
    const result = await client.completion(editor.value, request.position);
    editor.openCompletion(result.items ?? [], {
      prefix: request.prefix,
      onAccept: () => {},
    });
  } catch {
    /* a trap is reported by the client */
  }
}

async function gotoDefinition(position) {
  if (!client?.alive) return;
  try {
    const result = await client.definition(editor.value, position);
    if (!result) {
      banner('info', 'No definition here: this name is not bound in the file (built-ins like <code>plus</code> or <code>U</code> live in the language).');
      setTimeout(() => {
        if ($('banner').className === 'banner info') $('banner').hidden = true;
      }, 3500);
      return;
    }
    const start = offsetAt(editor.value, result.range.start);
    const end = Math.max(start + 1, offsetAt(editor.value, result.range.end));
    editor.selectRange(start, end);
    $('banner').hidden = true;
  } catch {
    /* a trap is reported by the client */
  }
}

// ---------------------------------------------------------------------------
// storage
// ---------------------------------------------------------------------------

function save() {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ preset: $('preset').value, text: editor.value }),
    );
  } catch {
    /* private mode: editing still works, it just is not remembered */
  }
}

function restore() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (typeof parsed.text !== 'string') return null;
    return parsed;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// start
// ---------------------------------------------------------------------------

function showNotBuilt(error) {
  $('server-status').textContent = 'wasm not built';
  banner(
    'warn',
    `<strong>The language server module is not built yet.</strong>
     <p>Run <code>node web/build.mjs</code> in the repository root, then reload this page.
     ${escapeHtml(error?.message ?? '')}</p>`,
  );
  $('run').disabled = true;
}

async function start() {
  try {
    wasm = await loadWasm();
  } catch (error) {
    showNotBuilt(error);
    return;
  }

  const saved = restore();
  const preset = saved?.preset && PRESETS[saved.preset] ? saved.preset : initialPreset;

  const select = $('preset');
  for (const [key, value] of Object.entries(PRESETS)) {
    const option = document.createElement('option');
    option.value = key;
    option.textContent = value.label;
    select.appendChild(option);
  }
  select.value = preset;

  editor = new CodeEditor($('editor'), { text: saved?.text ?? PRESETS[preset].text });

  client = new LspClient(wasm, {
    uri: 'file:///main.dpm',
    onNotification: (message) => {
      if (message.method === 'textDocument/publishDiagnostics') {
        renderDiagnostics(message.params.diagnostics ?? []);
      } else if (message.method === '$/trap') {
        if (message.params.recovered) {
          banner(
            'info',
            `<strong>The last request overflowed the browser's wasm stack.</strong>
             <p>The session restarted itself; the editor keeps working.
             ${escapeHtml(message.params.reason ?? '')}</p>`,
          );
          setTimeout(() => {
            if ($('banner').className === 'banner info') $('banner').hidden = true;
          }, 6000);
        } else {
          banner(
            'warn',
            `<strong>The wasm instance crashed (a Rust panic cannot unwind on <code>wasm32-unknown-unknown</code>).</strong>
             <p>Reload the page to start a fresh instance${message.params.panic ? ' and quote the panic below in a report' : ''}.</p>
             ${message.params.panic
              ? `<pre>${escapeHtml(message.params.panic)}</pre>`
              : `<p>${escapeHtml(message.params.reason ?? '')}</p>`}`,
          );
          $('run').disabled = true;
        }
      }
    },
    onLog: (entry) => {
      logEntries.push(entry);
      if (logVisible) renderLog();
    },
  });

  try {
    await client.start();
  } catch (error) {
    banner('warn', `<strong>initialize failed.</strong><p>${escapeHtml(error.message)}</p>`);
    return;
  }
  renderServerStatus();
  client.open(editor.value);
  await refresh(editor.value);
  await run('nf');

  // -- editor wiring -------------------------------------------------------

  editor.on('change', ({ text, completion }) => {
    save();
    clearTimeout(changeTimer);
    clearTimeout(hoverTimer);
    changeTimer = setTimeout(() => {
      client.change(text);
      refresh(text);
    }, 120);

    if (completion && /[A-Za-z0-9']/.test(text[completion.offset - 1] ?? '')) {
      clearTimeout(completionTimer);
      completionTimer = setTimeout(() => requestCompletion(completion, { automatic: true }), 160);
    }
  });

  editor.on('caret', ({ position, offset }) => {
    renderCursor(position);
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(async () => {
      if (!client.alive) return;
      try {
        const result = await client.hover(editor.value, position);
        const contents = result?.contents;
        if (contents) {
          editor.showHover(contents.value ?? String(contents), editor.rectAtOffset(offset));
        } else {
          editor.hideHover();
        }
      } catch {
        editor.hideHover();
      }
    }, 250);
  });

  editor.on('complete', (request) => requestCompletion(request, { automatic: false }));
  editor.on('definition', ({ position }) => gotoDefinition(position));
  editor.on('run', () => run(lastResultMode));

  // -- toolbar -------------------------------------------------------------

  select.addEventListener('change', () => {
    editor.setValue(PRESETS[select.value].text);
    save();
    clearTimeout(changeTimer);
    client.change(editor.value);
    refresh(editor.value);
    run(lastResultMode);
    history.replaceState(null, '', `?preset=${select.value}`);
  });

  $('run').addEventListener('click', () => run('nf'));
  $('type').addEventListener('click', () => run('type'));
  $('goto').addEventListener('click', () => gotoDefinition(editor.caretPosition()));
  $('complete').addEventListener('click', () => requestCompletion(editor.completionRequest(), { automatic: false }));
  $('reset').addEventListener('click', () => {
    editor.setValue(PRESETS[select.value].text);
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* ignore */
    }
    client.change(editor.value);
    refresh(editor.value);
    run(lastResultMode);
  });
  $('toggle-log').addEventListener('click', () => {
    logVisible = !logVisible;
    $('log-pane').hidden = !logVisible;
    $('toggle-log').classList.toggle('active', logVisible);
    if (logVisible) renderLog();
  });

  renderDiagnostics(client.diagnostics);
  window.addEventListener('beforeunload', save);
}

await start();
