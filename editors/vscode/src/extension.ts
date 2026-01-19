import { workspace, ExtensionContext, window, OutputChannel } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  State
} from 'vscode-languageclient/node';
import { execSync } from 'child_process';

let client: LanguageClient;
let outputChannel: OutputChannel;

function log(message: string) {
  const timestamp = new Date().toISOString();
  outputChannel.appendLine(`[${timestamp}] ${message}`);
}

export async function activate(context: ExtensionContext) {
  outputChannel = window.createOutputChannel('Hone Language Server');
  context.subscriptions.push(outputChannel);
  
  log('Hone extension activating...');
  
  // Check if hone binary is available
  let honePath: string | undefined;
  try {
    honePath = execSync('which hone', { encoding: 'utf8' }).trim();
    log(`Found hone binary at: ${honePath}`);
  } catch {
    log('ERROR: hone binary not found in PATH');
    log('Please install hone and ensure it is available in your PATH');
    outputChannel.show();
    return;
  }

  // Check hone version
  try {
    const version = execSync('hone --version', { encoding: 'utf8' }).trim();
    log(`Hone version: ${version}`);
  } catch (error) {
    log(`WARNING: Could not determine hone version: ${error}`);
  }

  const serverOptions: ServerOptions = {
    command: 'hone',
    args: ['lsp'],
    transport: TransportKind.stdio
  };

  log(`Server options: command='hone', args=['lsp'], transport=stdio`);

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'hone' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.hone')
    },
    outputChannel: outputChannel
  };

  log('Creating language client...');

  client = new LanguageClient(
    'hone',
    'Hone Language Server',
    serverOptions,
    clientOptions
  );

  client.onDidChangeState((event) => {
    const oldState = stateToString(event.oldState);
    const newState = stateToString(event.newState);
    log(`Client state changed: ${oldState} -> ${newState}`);
    
    if (event.newState === State.Stopped) {
      log('ERROR: Language server stopped unexpectedly');
      log('Check the Hone LSP server logs for more details:');
      log('  - Linux/macOS: ~/.local/state/hone/lsp.log');
      log('  - Windows: %LOCALAPPDATA%\\hone\\lsp.log');
      outputChannel.show();
    }
  });

  try {
    log('Starting language client...');
    await client.start();
    log('Language client started successfully');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    log(`ERROR: Failed to start language client: ${message}`);
    if (error instanceof Error && error.stack) {
      log(`Stack trace: ${error.stack}`);
    }
    outputChannel.show();
  }
}

function stateToString(state: State): string {
  switch (state) {
    case State.Stopped: return 'Stopped';
    case State.Starting: return 'Starting';
    case State.Running: return 'Running';
    default: return `Unknown(${state})`;
  }
}

export function deactivate(): Thenable<void> | undefined {
  log('Hone extension deactivating...');
  if (!client) {
    return undefined;
  }
  return client.stop();
}
