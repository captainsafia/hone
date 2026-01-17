import * as vscode from 'vscode';
import { HoneTestResult } from './hone';

const diagnosticCollection = vscode.languages.createDiagnosticCollection('hone');

/**
 * Update diagnostics based on test results
 */
export function updateDiagnostics(fileUri: vscode.Uri, result: HoneTestResult): void {
  const diagnostics: vscode.Diagnostic[] = [];
  
  for (const test of result.tests) {
    if (test.status === 'failed') {
      for (const run of test.runs) {
        if (run.status === 'failed') {
          for (const assertion of run.assertions) {
            if (assertion.status === 'failed') {
              const range = new vscode.Range(
                new vscode.Position(assertion.line - 1, 0),
                new vscode.Position(assertion.line - 1, Number.MAX_VALUE)
              );
              
              let message = `Assertion failed: ${assertion.expression}`;
              if (assertion.expected && assertion.actual) {
                message += `\nExpected: ${assertion.expected}\nActual: ${assertion.actual}`;
              }
              
              const diagnostic = new vscode.Diagnostic(
                range,
                message,
                vscode.DiagnosticSeverity.Error
              );
              
              diagnostic.source = 'hone';
              diagnostics.push(diagnostic);
            }
          }
        }
      }
    }
  }
  
  diagnosticCollection.set(fileUri, diagnostics);
}

/**
 * Clear diagnostics for a file
 */
export function clearDiagnostics(fileUri: vscode.Uri): void {
  diagnosticCollection.delete(fileUri);
}

/**
 * Clear all diagnostics
 */
export function clearAllDiagnostics(): void {
  diagnosticCollection.clear();
}

/**
 * Dispose the diagnostic collection
 */
export function disposeDiagnostics(): void {
  diagnosticCollection.dispose();
}
