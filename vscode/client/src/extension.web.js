// Browser extension host (VS Code for the Web): run the wasm language server
// inside a Web Worker and let `vscode-languageclient` talk to it.
//
// No native process, no WASI runtime, no extra extension: the worker loads the
// same `dpm_nbe_wasm.js` / `dpm_nbe_wasm_bg.wasm` pair the web IDE serves.

import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/browser';
import { registerCommands } from './commands.js';

const LANGUAGE = 'dpm-nbe';

export async function activate(context) {
  const output = vscode.window.createOutputChannel('dpm-nbe');
  const workerUri = vscode.Uri.joinPath(context.extensionUri, 'dist', 'web', 'worker.js');
  output.appendLine(`dpm-nbe: wasm language server = ${workerUri.toString(true)}`);

  // `type: 'module'` so the worker can `import()` the wasm-bindgen glue.
  const worker = new Worker(workerUri.toString(true), {
    type: 'module',
    name: 'dpm-nbe-lsp',
  });
  worker.addEventListener('error', (event) => {
    output.appendLine(`dpm-nbe: worker error: ${event.message ?? event}`);
  });

  // Note the argument order of the *browser* `LanguageClient`:
  // `(id, name, clientOptions, worker)` — unlike the Node one, the worker
  // comes last.
  const client = new LanguageClient(
    'dpm-nbe-lsp',
    'dpm-nbe language server',
    {
      documentSelector: [{ language: LANGUAGE }],
      outputChannel: output,
    },
    worker,
  );

  registerCommands(context, client, output);
  context.subscriptions.push(client, { dispose: () => worker.terminate() });

  try {
    await client.start();
  } catch (error) {
    void vscode.window.showErrorMessage(
      `dpm-nbe: cannot start the wasm language server (${error.message}). ` +
        'Build it with `node web/build.mjs`, then reinstall the extension.',
    );
  }
}

export function deactivate() {
  return undefined;
}
