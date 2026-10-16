// Desktop (Node) extension host: spawn the native `dpm-nbe-lsp` binary.
//
// The browser counterpart (`extension.web.js`) runs the *same* server compiled
// to WebAssembly in a worker; the language features and the protocol are
// identical because both link `dpm-nbe-lsp`.

import * as vscode from 'vscode';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import { registerCommands } from './commands.js';

const LANGUAGE = 'dpm-nbe';
const BINARY = process.platform === 'win32' ? 'dpm-nbe-lsp.exe' : 'dpm-nbe-lsp';

/** Where the server might be, best candidate first. */
function candidates() {
  const configured = vscode.workspace.getConfiguration(LANGUAGE).get('lsp.path');
  const list = [];
  if (configured) list.push(configured);
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    // `<root>/target/...` is where a workspace build puts it (`cargo build
    // --release -p dpm-nbe-lsp`); `<root>/lsp/target/...` is a build run from
    // inside `lsp/` itself.
    for (const profile of ['release', 'debug']) {
      list.push(vscode.Uri.joinPath(folder.uri, 'target', profile, BINARY).fsPath);
      list.push(vscode.Uri.joinPath(folder.uri, 'lsp', 'target', profile, BINARY).fsPath);
    }
  }
  // Last resort: whatever is on PATH.
  list.push(BINARY);
  return list;
}

function resolveCommand() {
  for (const candidate of candidates()) {
    if (path.isAbsolute(candidate)) {
      if (fs.existsSync(candidate)) return candidate;
    } else {
      return candidate;
    }
  }
  return BINARY;
}

export async function activate(context) {
  const output = vscode.window.createOutputChannel('dpm-nbe');
  const command = resolveCommand();
  output.appendLine(`dpm-nbe: language server = ${command}`);

  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };
  const client = new LanguageClient(
    'dpm-nbe-lsp',
    'dpm-nbe language server',
    serverOptions,
    {
      documentSelector: [{ language: LANGUAGE }],
      outputChannel: output,
      traceOutputChannel: output,
      // The server only needs the text of open documents.
      synchronize: {},
    },
  );

  registerCommands(context, client, output);
  context.subscriptions.push(client);

  try {
    await client.start();
  } catch (error) {
    const build = 'Build the language server';
    const choose = 'Set the path…';
    const choice = await vscode.window.showErrorMessage(
      `dpm-nbe: cannot start the language server (${error.message}). ` +
        'Build it with `cargo build --release -p dpm-nbe-lsp` or point ' +
        '`dpm-nbe.lsp.path` at the executable.',
      build,
      choose,
    );
    if (choice === build) {
      const terminal = vscode.window.createTerminal('dpm-nbe language server');
      terminal.show();
      terminal.sendText('cargo build --release -p dpm-nbe-lsp');
    } else if (choice === choose) {
      await vscode.commands.executeCommand('workbench.action.openSettings', 'dpm-nbe.lsp.path');
    }
  }
}

export function deactivate() {
  // The client is disposed through `context.subscriptions`.
  return undefined;
}
