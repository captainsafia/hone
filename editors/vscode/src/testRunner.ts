import * as vscode from 'vscode';
import { runHoneCommand, parseHoneJsonOutput, HoneTestResult } from './hone';

let sharedTerminal: vscode.Terminal | undefined;

/**
 * Get or create the shared Hone Tests terminal
 */
function getHoneTerminal(): vscode.Terminal {
  if (!sharedTerminal || sharedTerminal.exitStatus !== undefined) {
    sharedTerminal = vscode.window.createTerminal('Hone Tests');
  }
  return sharedTerminal;
}

/**
 * Run tests for a file
 */
export async function runFileTests(
  filePath: string,
  run: vscode.TestRun,
  testItems: vscode.TestItem[]
): Promise<void> {
  const terminal = getHoneTerminal();
  terminal.show();
  terminal.sendText('clear');
  
  // Mark all tests as queued
  for (const item of testItems) {
    run.enqueued(item);
  }
  
  try {
    // Run hone command
    const result = await runHoneCommand(filePath);
    
    if (result.exitCode !== 0 && !result.stdout) {
      // Command failed without JSON output
      for (const item of testItems) {
        run.failed(item, new vscode.TestMessage(result.stderr || 'Test execution failed'));
      }
      return;
    }
    
    // Try to parse JSON output
    const jsonResult = parseHoneJsonOutput(result.stdout);
    
    if (!jsonResult) {
      // Fallback to basic pass/fail based on exit code
      for (const item of testItems) {
        if (result.exitCode === 0) {
          run.passed(item);
        } else {
          run.failed(item, new vscode.TestMessage(result.stderr || 'Test failed'));
        }
      }
      return;
    }
    
    // Update test results from JSON
    updateTestResults(run, testItems, jsonResult);
    
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    for (const item of testItems) {
      run.failed(item, new vscode.TestMessage(message));
    }
  }
}

/**
 * Run a specific test by name
 */
export async function runTestByName(
  filePath: string,
  testName: string,
  run: vscode.TestRun,
  testItems: vscode.TestItem[]
): Promise<void> {
  const terminal = getHoneTerminal();
  terminal.show();
  terminal.sendText('clear');
  
  // Mark all test items as queued
  for (const item of testItems) {
    run.enqueued(item);
  }
  
  try {
    // Run hone command with test filter
    const result = await runHoneCommand(filePath, testName);
    
    if (result.exitCode !== 0 && !result.stdout) {
      for (const item of testItems) {
        run.failed(item, new vscode.TestMessage(result.stderr || 'Test execution failed'));
      }
      return;
    }
    
    const jsonResult = parseHoneJsonOutput(result.stdout);
    
    if (!jsonResult) {
      for (const item of testItems) {
        if (result.exitCode === 0) {
          run.passed(item);
        } else {
          run.failed(item, new vscode.TestMessage(result.stderr || 'Test failed'));
        }
      }
      return;
    }
    
    updateTestResults(run, testItems, jsonResult);
    
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    for (const item of testItems) {
      run.failed(item, new vscode.TestMessage(message));
    }
  }
}

/**
 * Update test results from JSON output
 */
function updateTestResults(
  run: vscode.TestRun,
  testItems: vscode.TestItem[],
  result: HoneTestResult
): void {
  // Create a map of test names to results
  const testResultMap = new Map<string, any>();
  for (const test of result.tests) {
    testResultMap.set(test.name, test);
  }
  
  for (const item of testItems) {
    const testName = item.label;
    const testResult = testResultMap.get(testName);
    
    if (!testResult) {
      run.skipped(item);
      continue;
    }
    
    const durationMs = testResult.duration_ms;
    
    if (testResult.status === 'passed') {
      run.passed(item, durationMs);
    } else {
      // Collect failure messages
      const messages: vscode.TestMessage[] = [];
      
      for (const runItem of testResult.runs) {
        if (runItem.status === 'failed') {
          for (const assertion of runItem.assertions) {
            if (assertion.status === 'failed') {
              const message = new vscode.TestMessage(
                `Assertion failed: ${assertion.expression}`
              );
              
              if (assertion.expected && assertion.actual) {
                message.expectedOutput = assertion.expected;
                message.actualOutput = assertion.actual;
              }
              
              if (item.uri && assertion.line) {
                message.location = new vscode.Location(
                  item.uri,
                  new vscode.Position(assertion.line - 1, 0)
                );
              }
              
              messages.push(message);
            }
          }
        }
      }
      
      run.failed(item, messages.length > 0 ? messages : new vscode.TestMessage('Test failed'), durationMs);
    }
  }
}

/**
 * Dispose the shared terminal
 */
export function disposeTerminal(): void {
  if (sharedTerminal) {
    sharedTerminal.dispose();
    sharedTerminal = undefined;
  }
}
