import * as vscode from 'vscode';
import * as fs from 'fs/promises';

export interface TestBlock {
  name: string;
  line: number;
  runs: RunCommand[];
}

export interface RunCommand {
  name: string | null;
  command: string;
  line: number;
}

/**
 * Discover all .hone files in the workspace
 */
export async function discoverTestFiles(): Promise<vscode.Uri[]> {
  const files = await vscode.workspace.findFiles('**/*.hone', '**/node_modules/**');
  return files;
}

/**
 * Parse a .hone file to extract TEST blocks and RUN commands
 */
export async function parseHoneFile(filePath: string): Promise<TestBlock[]> {
  try {
    const content = await fs.readFile(filePath, 'utf-8');
    const lines = content.split('\n');
    
    const testBlocks: TestBlock[] = [];
    let currentTest: TestBlock | null = null;
    
    // Regex patterns
    const testPattern = /^TEST\s+"([^"]+)"/;
    const runPattern = /^RUN(?:\s+(\w+):)?\s+(.+)$/;
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      const lineNumber = i + 1; // 1-indexed
      
      // Check for TEST block
      const testMatch = line.match(testPattern);
      if (testMatch) {
        currentTest = {
          name: testMatch[1],
          line: lineNumber,
          runs: [],
        };
        testBlocks.push(currentTest);
        continue;
      }
      
      // Check for RUN command (only inside a TEST block)
      if (currentTest) {
        const runMatch = line.match(runPattern);
        if (runMatch) {
          const name = runMatch[1] || null;
          const command = runMatch[2];
          currentTest.runs.push({
            name,
            command,
            line: lineNumber,
          });
        }
      }
    }
    
    return testBlocks;
  } catch (error) {
    console.error(`Failed to parse hone file ${filePath}:`, error);
    return [];
  }
}

/**
 * Create test items for a file
 */
export function createTestItemsForFile(
  controller: vscode.TestController,
  fileUri: vscode.Uri,
  testBlocks: TestBlock[]
): vscode.TestItem {
  const fileItem = controller.createTestItem(
    fileUri.toString(),
    fileUri.fsPath.split('/').pop() || fileUri.fsPath,
    fileUri
  );
  
  fileItem.canResolveChildren = true;
  
  for (const testBlock of testBlocks) {
    const testId = `${fileUri.toString()}:${testBlock.line}`;
    const testItem = controller.createTestItem(
      testId,
      testBlock.name,
      fileUri
    );
    
    testItem.range = new vscode.Range(
      new vscode.Position(testBlock.line - 1, 0),
      new vscode.Position(testBlock.line - 1, 0)
    );
    
    testItem.canResolveChildren = true;
    
    for (const run of testBlock.runs) {
      const runId = `${testId}:${run.line}`;
      const runLabel = run.name || run.command;
      const runItem = controller.createTestItem(
        runId,
        runLabel,
        fileUri
      );
      
      runItem.range = new vscode.Range(
        new vscode.Position(run.line - 1, 0),
        new vscode.Position(run.line - 1, 0)
      );
      
      testItem.children.add(runItem);
    }
    
    fileItem.children.add(testItem);
  }
  
  return fileItem;
}
