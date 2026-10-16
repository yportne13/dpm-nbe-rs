// A tiny Language Server Protocol client for the browser build.
//
// The module has no DOM dependency on purpose: `web/tools/client-smoke.mjs`
// drives this exact class in Node against the real wasm module, so the client
// half of the conversation is covered by tests too.

/** Byte/UTF-16 offset helpers (LSP positions are UTF-16 code units). */
export function positionAt(text, offset) {
  const clamped = Math.max(0, Math.min(offset, text.length));
  const before = text.slice(0, clamped);
  const line = (before.match(/\n/g) ?? []).length;
  const lineStart = before.lastIndexOf('\n') + 1;
  return { line, character: clamped - lineStart };
}

export function offsetAt(text, position) {
  const lines = text.split('\n');
  let offset = 0;
  for (let i = 0; i < position.line && i < lines.length; i += 1) {
    offset += lines[i].length + 1;
  }
  return Math.min(offset + position.character, text.length);
}

/**
 * Drives one `LspSession` and exposes the editor-facing API.
 *
 * Notifications the server pushes (`textDocument/publishDiagnostics`) are kept
 * in `diagnostics` and forwarded to `onNotification`, so the UI can render them
 * without polling.
 */
export class LspClient {
  constructor(wasm, options = {}) {
    this.wasm = wasm;
    this.uri = options.uri ?? 'file:///untitled.dpm';
    this.languageId = options.languageId ?? 'dpm-nbe';
    this.onNotification = options.onNotification ?? (() => {});
    this.onLog = options.onLog ?? (() => {});

    this.session = null;
    this.serverInfo = null;
    this.capabilities = null;
    this.version = 0;
    this.nextId = 1;
    this.pending = new Map();
    this.diagnostics = [];
    this.diagnosticsUri = null;
    this.alive = false;
    this.deadReason = null;
  }

  /** A trap leaves the wasm instance unusable; every later call is refused. */
  die(error) {
    this.alive = false;
    this.deadReason = String(error?.message ?? error);
    for (const pending of this.pending.values()) {
      pending.reject(new Error(this.deadReason));
    }
    this.pending.clear();
    this.onNotification({ method: '$/trap', params: { reason: this.deadReason } });
  }

  /** Hand one message to the module and fan the replies out. */
  transmit(message) {
    if (!this.alive) {
      throw new Error(`language server is gone: ${this.deadReason}`);
    }
    this.onLog({ direction: 'client→server', message });
    let replies;
    try {
      replies = JSON.parse(this.session.send(JSON.stringify(message)));
    } catch (error) {
      this.die(error);
      throw error;
    }
    this.onLog({ direction: 'server→client', message: replies });
    this.dispatch(replies);
    return replies;
  }

  dispatch(replies) {
    for (const reply of replies) {
      if (reply.method === 'textDocument/publishDiagnostics') {
        this.diagnosticsUri = reply.params.uri;
        if (reply.params.uri === this.uri) {
          this.diagnostics = reply.params.diagnostics ?? [];
        }
        this.onNotification(reply);
        continue;
      }
      if (reply.id !== undefined && this.pending.has(reply.id)) {
        const pending = this.pending.get(reply.id);
        this.pending.delete(reply.id);
        if (reply.error) {
          pending.reject(new Error(`${pending.method}: ${reply.error.message}`));
        } else {
          pending.resolve(reply.result);
        }
        continue;
      }
      this.onNotification(reply);
    }
  }

  request(method, params) {
    return new Promise((resolve, reject) => {
      const id = this.nextId;
      this.nextId += 1;
      this.pending.set(id, { resolve, reject, method });
      try {
        this.transmit({ jsonrpc: '2.0', id, method, params });
      } catch (error) {
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  notify(method, params) {
    try {
      this.transmit({ jsonrpc: '2.0', method, params });
      return true;
    } catch {
      return false;
    }
  }

  // -- lifecycle -----------------------------------------------------------

  async start() {
    this.session = new this.wasm.LspSession();
    this.alive = true;
    const result = await this.request('initialize', {
      processId: null,
      clientInfo: { name: 'dpm-nbe-web', version: '1.0.0' },
      capabilities: {
        textDocument: {
          hover: { contentFormat: ['markdown', 'plaintext'] },
          completion: { completionItem: { documentationFormat: ['markdown'] } },
          publishDiagnostics: {},
        },
      },
    });
    this.serverInfo = result.serverInfo ?? null;
    this.capabilities = result.capabilities ?? null;
    this.notify('initialized', {});
    return result;
  }

  open(text) {
    this.version = 1;
    return this.notify('textDocument/didOpen', {
      textDocument: {
        uri: this.uri,
        languageId: this.languageId,
        version: this.version,
        text,
      },
    });
  }

  change(text) {
    this.version += 1;
    return this.notify('textDocument/didChange', {
      textDocument: { uri: this.uri, version: this.version },
      contentChanges: [{ text }],
    });
  }

  save(text) {
    return this.notify('textDocument/didSave', {
      textDocument: { uri: this.uri },
      text,
    });
  }

  async shutdown() {
    if (!this.alive) return null;
    const result = await this.request('shutdown', null);
    this.notify('exit', null);
    this.alive = false;
    return result;
  }

  // -- features ------------------------------------------------------------

  hover(text, position) {
    return this.request('textDocument/hover', {
      textDocument: { uri: this.uri },
      text,
      position,
    });
  }

  completion(text, position) {
    return this.request('textDocument/completion', {
      textDocument: { uri: this.uri },
      text,
      position,
    });
  }

  definition(text, position) {
    return this.request('textDocument/definition', {
      textDocument: { uri: this.uri },
      text,
      position,
    });
  }

  documentSymbols(text) {
    return this.request('textDocument/documentSymbol', {
      textDocument: { uri: this.uri },
      text,
    });
  }

  tokens(text) {
    return this.request('dpm-nbe/tokens', { textDocument: { uri: this.uri }, text });
  }

  evaluate(text, mode = 'nf') {
    return this.request('dpm-nbe/evaluate', {
      textDocument: { uri: this.uri },
      text,
      mode,
    });
  }
}
