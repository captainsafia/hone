import * as vscode from 'vscode';
import { discoverTestFiles, parseHoneFile } from './testDiscovery';
import { runFileTests, runTestByName, disposeTerminal } from './testRunner';
import { findHoneBinary, promptInstallHone, parseHoneJsonOutput, runHoneCommand } from './hone';
import { updateDiagnostics, clearDiagnostics, disposeDiagnostics, clearAllDiagnostics } from './diagnostics';
import { HoneCodeLensProvider } from './codeLens';

export class HoneTestController {
  private controller: vscode.TestController;
  private fileWatcher: vscode.FileSystemWatcher;
  private codeLensProvider: HoneCodeLensProvider;
  private disposables: vscode.Disposable[] = [];
  private _runProfile: vscode.TestRunProfile;

  constructor(_context: vscode.ExtensionContext) {
    // Create test controller
    this.controller = vscode.tests.createTestController(
      'honeTestController',
      'Hone Tests'
    );
    
    this.disposables.push(this.controller);

    // Set up resolve handler for lazy loading
    this.controller.resolveHandler = async (item) => {
      if (!item) {
        // Root level - discover all test files
        await this.discoverAllTests();
      } else if (item.canResolveChildren) {
        // Lazy load children for a specific file
        await this.resolveTestFile(item);
      }
    };

    // Create run profile
    this._runProfile = this.controller.createRunProfile(
      'Run',
      vscode.TestRunProfileKind.Run,
      async (request, token) => {
        await this.runTests(request, token);
      },
      true
    );

    // Watch for .hone file changes
    this.fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.hone');
    
    this.fileWatcher.onDidCreate(async (uri) => {
      await this.addTestFile(uri);
    });
    
    this.fileWatcher.onDidChange(async (uri) => {
      await this.refreshTestFile(uri);
    });
    
    this.fileWatcher.onDidDelete((uri) => {
      this.controller.items.delete(uri.toString());
    });
    
    this.disposables.push(this.fileWatcher);

    // Register CodeLens provider
    this.codeLensProvider = new HoneCodeLensProvider(this.controller);
    const codeLensDisposable = vscode.languages.registerCodeLensProvider(
      { language: 'hone' },
      this.codeLensProvider
    );
    this.disposables.push(codeLensDisposable);

    // Register commands for CodeLens
    this.disposables.push(
      vscode.commands.registerCommand('hone.runTest', async (uri: vscode.Uri, testName: string) => {
        await this.runTestByName(uri, testName);
      })
    );

    this.disposables.push(
      vscode.commands.registerCommand('hone.revealTest', async (uri: vscode.Uri, testName: string) => {
        await this.revealTest(uri, testName);
      })
    );

    // Check for hone binary
    this.checkHoneBinary();

    // Initial discovery
    this.discoverAllTests();
  }

  private async checkHoneBinary(): Promise<void> {
    const honePath = await findHoneBinary();
    if (!honePath) {
      await promptInstallHone();
    }
  }

  private async discoverAllTests(): Promise<void> {
    const files = await discoverTestFiles();
    
    for (const fileUri of files) {
      await this.addTestFile(fileUri);
    }
  }

  private async addTestFile(fileUri: vscode.Uri): Promise<void> {
    // Create file item without parsing yet (lazy loading)
    const existingItem = this.controller.items.get(fileUri.toString());
    if (!existingItem) {
      const fileItem = this.controller.createTestItem(
        fileUri.toString(),
        fileUri.fsPath.split('/').pop() || fileUri.fsPath,
        fileUri
      );
      fileItem.canResolveChildren = true;
      this.controller.items.add(fileItem);
    }
  }

  private async refreshTestFile(fileUri: vscode.Uri): Promise<void> {
    const existingItem = this.controller.items.get(fileUri.toString());
    if (existingItem) {
      // Re-parse the file
      existingItem.children.replace([]);
      await this.resolveTestFile(existingItem);
      this.codeLensProvider.refresh();
    }
  }

  private async resolveTestFile(fileItem: vscode.TestItem): Promise<void> {
    if (!fileItem.uri) {
      return;
    }

    const testBlocks = await parseHoneFile(fileItem.uri.fsPath);
    
    // Clear existing children
    fileItem.children.replace([]);
    
    // Add test blocks and runs
    for (const testBlock of testBlocks) {
      const testId = `${fileItem.id}:${testBlock.line}`;
      const testItem = this.controller.createTestItem(
        testId,
        testBlock.name,
        fileItem.uri
      );
      
      testItem.range = new vscode.Range(
        new vscode.Position(testBlock.line - 1, 0),
        new vscode.Position(testBlock.line - 1, 0)
      );
      
      testItem.canResolveChildren = false;
      
      // Add RUN commands as children
      for (const run of testBlock.runs) {
        const runId = `${testId}:${run.line}`;
        const runLabel = run.name || run.command;
        const runItem = this.controller.createTestItem(
          runId,
          runLabel,
          fileItem.uri
        );
        
        runItem.range = new vscode.Range(
          new vscode.Position(run.line - 1, 0),
          new vscode.Position(run.line - 1, 0)
        );
        
        testItem.children.add(runItem);
      }
      
      fileItem.children.add(testItem);
    }
  }

  private async runTests(
    request: vscode.TestRunRequest,
    token: vscode.CancellationToken
  ): Promise<void> {
    const run = this.controller.createTestRun(request);
    
    try {
      if (!request.include) {
        // Run all tests
        const allTests = this.collectAllTests();
        await this.executeTests(run, allTests, token);
      } else {
        // Run specific tests
        await this.executeTests(run, request.include, token);
      }
    } finally {
      run.end();
    }
  }

  private collectAllTests(): vscode.TestItem[] {
    const tests: vscode.TestItem[] = [];
    this.controller.items.forEach((item) => {
      tests.push(item);
    });
    return tests;
  }

  private async executeTests(
    run: vscode.TestRun,
    tests: readonly vscode.TestItem[],
    token: vscode.CancellationToken
  ): Promise<void> {
    for (const test of tests) {
      if (token.isCancellationRequested) {
        run.skipped(test);
        continue;
      }

      // Determine test granularity
      if (!test.uri) {
        continue;
      }

      // Check if this is a file-level test
      if (test.children.size > 0 && !test.id.includes(':')) {
        // File-level test - collect all child tests
        const childTests: vscode.TestItem[] = [];
        test.children.forEach((child) => childTests.push(child));
        await this.executeFileTests(run, test.uri, childTests);
      } else if (test.children.size > 0) {
        // TEST block level
        await this.executeTestBlock(run, test);
      } else {
        // RUN level - execute parent TEST block
        const parentId = test.id.substring(0, test.id.lastIndexOf(':'));
        const parent = this.findTestItemById(parentId);
        if (parent) {
          await this.executeTestBlock(run, parent);
        }
      }
    }
  }

  private async executeFileTests(
    run: vscode.TestRun,
    fileUri: vscode.Uri,
    testItems: vscode.TestItem[]
  ): Promise<void> {
    clearDiagnostics(fileUri);
    
    try {
      await runFileTests(fileUri.fsPath, run, testItems);
      
      // Try to get JSON results for diagnostics
      const result = await runHoneCommand(fileUri.fsPath);
      if (result.stdout) {
        const jsonResult = parseHoneJsonOutput(result.stdout);
        if (jsonResult) {
          updateDiagnostics(fileUri, jsonResult);
        }
      }
    } catch (error) {
      console.error('Error running file tests:', error);
    }
  }

  private async executeTestBlock(
    run: vscode.TestRun,
    testItem: vscode.TestItem
  ): Promise<void> {
    if (!testItem.uri) {
      return;
    }

    const testName = testItem.label;
    const fileUri = testItem.uri;
    
    clearDiagnostics(fileUri);
    
    try {
      await runTestByName(fileUri.fsPath, testName, run, [testItem]);
      
      // Try to get JSON results for diagnostics
      const result = await runHoneCommand(fileUri.fsPath, testName);
      if (result.stdout) {
        const jsonResult = parseHoneJsonOutput(result.stdout);
        if (jsonResult) {
          updateDiagnostics(fileUri, jsonResult);
        }
      }
    } catch (error) {
      console.error('Error running test block:', error);
    }
  }

  private findTestItemById(id: string): vscode.TestItem | undefined {
    // Parse ID to find the item
    const parts = id.split(':');
    const fileId = parts[0];
    
    const fileItem = this.controller.items.get(fileId);
    if (!fileItem) {
      return undefined;
    }
    
    if (parts.length === 1) {
      return fileItem;
    }
    
    if (parts.length === 2) {
      // TEST level
      let found: vscode.TestItem | undefined;
      fileItem.children.forEach((child) => {
        if (child.id === id) {
          found = child;
        }
      });
      return found;
    }
    
    // RUN level
    const testId = `${parts[0]}:${parts[1]}`;
    let testItem: vscode.TestItem | undefined;
    fileItem.children.forEach((child) => {
      if (child.id === testId) {
        testItem = child;
      }
    });
    
    if (!testItem) {
      return undefined;
    }
    
    let found: vscode.TestItem | undefined;
    testItem.children.forEach((child) => {
      if (child.id === id) {
        found = child;
      }
    });
    return found;
  }

  private async runTestByName(uri: vscode.Uri, testName: string): Promise<void> {
    // Find the test item
    const fileItem = this.controller.items.get(uri.toString());
    if (!fileItem) {
      return;
    }

    let testItem: vscode.TestItem | undefined;
    fileItem.children.forEach((child) => {
      if (child.label === testName) {
        testItem = child;
      }
    });

    if (!testItem) {
      return;
    }

    // Create a test run request
    const request = new vscode.TestRunRequest([testItem]);
    await this.runTests(request, new vscode.CancellationTokenSource().token);
  }

  private async revealTest(uri: vscode.Uri, testName: string): Promise<void> {
    // Find the test item
    const fileItem = this.controller.items.get(uri.toString());
    if (!fileItem) {
      return;
    }

    // Ensure children are loaded
    if (fileItem.children.size === 0) {
      await this.resolveTestFile(fileItem);
    }

    let testItem: vscode.TestItem | undefined;
    fileItem.children.forEach((child) => {
      if (child.label === testName) {
        testItem = child;
      }
    });

    if (testItem) {
      // Reveal in Test Explorer
      await vscode.commands.executeCommand('vscode.revealTestInExplorer', testItem);
    }
  }

  public dispose(): void {
    clearAllDiagnostics();
    disposeDiagnostics();
    disposeTerminal();
    
    for (const disposable of this.disposables) {
      disposable.dispose();
    }
  }
}
