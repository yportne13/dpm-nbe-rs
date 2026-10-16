// Commands shared by the desktop and the browser extension host.
//
// Both go through the *language server* rather than calling into the checker
// directly: `dpm-nbe/evaluate` is a server extension request, so the result
// shown here is produced by exactly the code that produces the diagnostics.

import * as vscode from 'vscode';

const LANGUAGE = 'dpm-nbe';

async function activeDocument() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== LANGUAGE) {
    await vscode.window.showInformationMessage(
      `Open a .dpm file first: the active editor is not a ${LANGUAGE} document.`,
    );
    return null;
  }
  return editor.document;
}

function evaluate(client, output, mode) {
  return async () => {
    const document = await activeDocument();
    if (!document) return;
    let result;
    try {
      result = await client.sendRequest('dpm-nbe/evaluate', {
        textDocument: { uri: document.uri.toString() },
        mode,
      });
    } catch (error) {
      void vscode.window.showErrorMessage(`dpm-nbe: ${error.message}`);
      return;
    }
    output.show(true);
    const title = mode === 'type' ? 'type' : 'normal form and type';
    output.appendLine(`--- ${vscode.workspace.asRelativePath(document.uri)} (${title}) ---`);
    output.appendLine(result.ok ? result.output : result.error);
    output.appendLine('');
  };
}

/**
 * @param {vscode.ExtensionContext} context
 * @param {import('vscode-languageclient').LanguageClient} client
 * @param {vscode.OutputChannel} output
 */
export function registerCommands(context, client, output) {
  context.subscriptions.push(
    vscode.commands.registerCommand('dpm-nbe.showResult', evaluate(client, output, 'nf')),
    vscode.commands.registerCommand('dpm-nbe.showType', evaluate(client, output, 'type')),
    vscode.commands.registerCommand('dpm-nbe.restartServer', async () => {
      output.appendLine('dpm-nbe: restarting the language server');
      try {
        await client.restart();
      } catch (error) {
        void vscode.window.showErrorMessage(`dpm-nbe: restart failed: ${error.message}`);
      }
    }),
  );
}
