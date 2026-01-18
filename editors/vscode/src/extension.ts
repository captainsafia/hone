import { workspace, ExtensionContext, window } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
  const serverOptions: ServerOptions = {
    command: 'hone',
    args: ['lsp'],
    transport: TransportKind.stdio
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'hone' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.hone')
    }
  };

  client = new LanguageClient(
    'hone',
    'Hone Language Server',
    serverOptions,
    clientOptions
  );

  try {
    await client.start();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    window.showErrorMessage(
      `Failed to start Hone language server. Ensure 'hone' is installed and available in your PATH. Error: ${message}`
    );
  }
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
