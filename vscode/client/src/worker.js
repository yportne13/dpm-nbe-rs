// The Web Worker that hosts the wasm language server in VS Code for the Web.
//
// The worker receives JSON-RPC objects from `vscode-languageclient` (which
// drives a Worker by postMessage) and answers with the messages the server
// produced.  The module itself is built by `web/build.mjs` and copied next to
// this file by `scripts/sync-wasm.mjs`.

import { startTransport } from './transport.js';

const MEDIA = new URL('../media/', self.location.href);

// `initialize` arrives while the wasm module is still being fetched, so buffer
// everything until the transport is listening — a message event with no
// listener would be dropped.
const buffered = [];
let deliver = (message) => buffered.push(message);
self.addEventListener('message', (event) => deliver(event.data));

async function loadWasm() {
  // The glue is generated with `wasm-bindgen --target web`, i.e. it is an ES
  // module; the worker is created with `{ type: 'module' }` so it can be
  // imported directly.  It is loaded at run time (not bundled) so that the
  // module the web IDE serves and the module the extension runs stay byte for
  // byte the same file.
  const glue = await import(new URL('dpm_nbe_wasm.js', MEDIA).href);
  const response = await fetch(new URL('dpm_nbe_wasm_bg.wasm', MEDIA));
  if (!response.ok) {
    throw new Error(`cannot fetch the wasm module: ${response.status} ${response.statusText}`);
  }
  await glue.default({ module_or_path: await response.arrayBuffer() });
  return glue;
}

try {
  await startTransport({
    loadWasm,
    postMessage: (message) => self.postMessage(message),
    subscribe: (handler) => {
      deliver = handler;
      for (const message of buffered.splice(0)) handler(message);
      return () => {
        deliver = (message) => buffered.push(message);
      };
    },
  });
} catch (error) {
  self.postMessage({
    jsonrpc: '2.0',
    id: null,
    error: {
      code: -32000,
      message: `cannot start the dpm-nbe language server: ${error?.message ?? error}`,
    },
  });
}
