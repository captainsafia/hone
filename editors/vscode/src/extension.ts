import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration('hone');
  const honePath = config.get<string>('lsp.path', 'hone');
  
  const serverOptions: ServerOptions = {
    command: honePath,
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

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
