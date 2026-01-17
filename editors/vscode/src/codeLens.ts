import * as vscode from 'vscode';

export class HoneCodeLensProvider implements vscode.CodeLensProvider {
  private _onDidChangeCodeLenses = new vscode.EventEmitter<void>();
  public readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

  constructor(_controller: vscode.TestController) {}

  public refresh(): void {
    this._onDidChangeCodeLenses.fire();
  }

  public provideCodeLenses(
    document: vscode.TextDocument
  ): vscode.CodeLens[] | Thenable<vscode.CodeLens[]> {
    const codeLenses: vscode.CodeLens[] = [];
    const text = document.getText();
    const lines = text.split('\n');

    // Pattern to match TEST blocks
    const testPattern = /^TEST\s+"([^"]+)"/;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      const match = line.match(testPattern);

      if (match) {
        const testName = match[1];
        const range = new vscode.Range(
          new vscode.Position(i, 0),
          new vscode.Position(i, lines[i].length)
        );

        // "Run Test" CodeLens
        const runCommand: vscode.Command = {
          title: '▶ Run Test',
          command: 'hone.runTest',
          arguments: [document.uri, testName],
        };

        codeLenses.push(new vscode.CodeLens(range, runCommand));

        // "View in Explorer" CodeLens
        const viewCommand: vscode.Command = {
          title: '🔍 View in Explorer',
          command: 'hone.revealTest',
          arguments: [document.uri, testName],
        };

        codeLenses.push(new vscode.CodeLens(range, viewCommand));
      }
    }

    return codeLenses;
  }
}
