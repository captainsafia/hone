import * as vscode from 'vscode';
import { HoneTestController } from './testController';

let testController: HoneTestController | undefined;

export function activate(context: vscode.ExtensionContext): void {
  console.log('Hone extension activating...');

  // Create and register the test controller
  testController = new HoneTestController(context);
  
  console.log('Hone extension activated');
}

export function deactivate(): void {
  if (testController) {
    testController.dispose();
    testController = undefined;
  }
}
