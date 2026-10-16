// The language server, bridged into a Worker.
//
// Deliberately free of Worker-specific globals: `startTransport` takes the
// message plumbing as arguments, so `client/test/transport.test.mjs` can drive
// the *same* code in Node with an in-memory channel and the wasm module loaded
// from disk.  `worker.js` is then a five-line shell.

/**
 * @param {object} options
 * @param {() => Promise<any>} options.loadWasm  Loads and initialises the module.
 * @param {(message: any) => void} options.postMessage  Sends one JSON-RPC message out.
 * @param {(handler: (message: any) => void) => () => void} options.subscribe
 *        Receives incoming JSON-RPC messages; returns an unsubscribe function.
 * @param {(direction: string, message: any) => void} [options.onLog]
 * @returns {Promise<{ session: any, module: any, send: (m: any) => any[] }>}
 */
export async function startTransport({ loadWasm, postMessage, subscribe, onLog }) {
  const module = await loadWasm();
  const session = new module.LspSession();

  const send = (message) => {
    if (message === null || typeof message !== 'object') {
      // Not a JSON-RPC message: reject it the way the protocol says to.
      postMessage({
        jsonrpc: '2.0',
        id: null,
        error: { code: -32600, message: 'invalid request: not a JSON-RPC message' },
      });
      return [];
    }
    let replies = [];
    try {
      replies = JSON.parse(session.send(JSON.stringify(message)));
    } catch (error) {
      // `wasm32-unknown-unknown` cannot unwind, so a Rust panic aborts the
      // instance and every later call throws.  Answer this one request with an
      // error rather than leaving the editor waiting forever.
      postMessage({
        jsonrpc: '2.0',
        id: message.id ?? null,
        error: {
          code: -32603,
          message: `dpm-nbe language server crashed: ${error?.message ?? error}`,
        },
      });
      return [];
    }
    if (onLog) onLog('receive', message);
    for (const reply of replies) {
      if (onLog) onLog('send', reply);
      postMessage(reply);
    }
    return replies;
  };

  const unsubscribe = subscribe(send);
  return { module, session, send, dispose: unsubscribe };
}
