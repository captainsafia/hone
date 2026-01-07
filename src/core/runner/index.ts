/**
 * Test Runner Orchestration
 *
 * Main entry point for running hone tests.
 */

import { readFile, stat } from "node:fs/promises";
import { resolve, dirname, basename } from "node:path";
import { glob } from "glob";
import {
  parseFile,
  type ParsedFile,
  type ASTNode,
  type AssertNode,
} from "../parser/index.ts";
import {
  ShellSession,
  createShellConfig,
  type RunResult,
} from "./shell.ts";
import {
  DefaultReporter,
  type Reporter,
  type TestFailure,
  type TestResults,
} from "./reporter.ts";
import {
  evaluateOutputPredicate,
  getOutputValue,
  evaluateExitCodePredicate,
  evaluateDurationPredicate,
  evaluateFilePredicate,
  type AssertionResult,
} from "../assertions/index.ts";

export { DefaultReporter, type Reporter, type TestResults };

/**
 * Runner options
 */
export interface RunnerOptions {
  shell?: string;
  verbose?: boolean;
  reporter?: Reporter;
  cwd?: string;
}

/**
 * Run result for a single file
 */
interface FileRunResult {
  filename: string;
  passed: boolean;
  assertionsPassed: number;
  assertionsFailed: number;
  failure?: TestFailure;
}

/**
 * Run tests from paths, directories, or glob patterns
 */
export async function runTests(
  patterns: string[],
  options: RunnerOptions = {}
): Promise<TestResults> {
  const reporter = options.reporter ?? new DefaultReporter({ verbose: options.verbose });
  const cwd = options.cwd ?? process.cwd();

  // Resolve files from all patterns
  const allFiles: string[] = [];
  for (const pattern of patterns) {
    const files = await resolveFiles(pattern, cwd);
    allFiles.push(...files);
  }

  const files = [...new Set(allFiles)].sort();

  if (files.length === 0) {
    reporter.onWarning(`No test files found matching: ${patterns.join(", ")}`);
    return {
      totalFiles: 0,
      passedFiles: 0,
      failedFiles: 0,
      totalAssertions: 0,
      passedAssertions: 0,
      failedAssertions: 0,
      failures: [],
    };
  }

  // Parse all files first (can be done in parallel)
  const parseResults = await Promise.all(
    files.map(async (file) => {
      const content = await readFile(file, "utf-8");
      return { file, result: parseFile(content, file) };
    })
  );

  // Collect parse errors
  const parseFailures: TestFailure[] = [];
  const validFiles: Array<{ file: string; parsed: ParsedFile }> = [];

  for (const { file, result } of parseResults) {
    if (!result.success) {
      reporter.onParseErrors(result.errors);
      for (const error of result.errors) {
        parseFailures.push({
          filename: error.filename,
          line: error.line,
          error: error.message,
        });
      }
    } else {
      // Report warnings
      for (const warning of result.file.warnings) {
        reporter.onWarning(
          `${warning.filename}:${warning.line} :: ${warning.message}`
        );
      }
      validFiles.push({ file, parsed: result.file });
    }
  }

  // Run each file sequentially
  const results: FileRunResult[] = [];

  for (const { file, parsed } of validFiles) {
    const result = await runFile(parsed, file, options, reporter);
    results.push(result);
  }

  // Compile final results
  const totalAssertions = results.reduce(
    (sum, r) => sum + r.assertionsPassed + r.assertionsFailed,
    0
  );
  const passedAssertions = results.reduce((sum, r) => sum + r.assertionsPassed, 0);
  const failedAssertions = results.reduce((sum, r) => sum + r.assertionsFailed, 0);
  const failures = [
    ...parseFailures,
    ...results.filter((r) => r.failure).map((r) => r.failure!),
  ];

  const testResults: TestResults = {
    totalFiles: files.length,
    passedFiles: results.filter((r) => r.passed).length,
    failedFiles: files.length - results.filter((r) => r.passed).length,
    totalAssertions,
    passedAssertions,
    failedAssertions,
    failures,
  };

  reporter.onSummary(testResults);

  return testResults;
}

/**
 * Resolve files from a pattern, file path, or directory
 */
async function resolveFiles(pattern: string, cwd: string): Promise<string[]> {
  const resolved = resolve(cwd, pattern);

  const stats = await stat(resolved).catch(() => null);

  if (stats?.isFile() && pattern.endsWith(".hone")) {
    return [resolved];
  }

  if (stats?.isDirectory()) {
    const dirMatches = await glob("**/*.hone", {
      cwd: resolved,
      absolute: true,
      nodir: true,
    });
    return dirMatches.sort();
  }

  // Use glob for pattern matching
  const matches = await glob(pattern, {
    cwd,
    absolute: true,
    nodir: true,
  });

  return matches.sort();
}

/**
 * Run a single test file
 */
async function runFile(
  parsed: ParsedFile,
  filename: string,
  options: RunnerOptions,
  reporter: Reporter
): Promise<FileRunResult> {
  const cwd = options.cwd ?? dirname(filename);

  reporter.onFileStart(basename(filename));

  // Create shell config from pragmas
  const shellConfig = createShellConfig(
    parsed.pragmas,
    filename,
    cwd,
    options.shell
  );

  // Group nodes by TEST block - each TEST gets its own shell session
  const testBlocks = groupNodesByTest(parsed.nodes);

  let totalAssertionsPassed = 0;
  let failure: TestFailure | undefined;

  for (const block of testBlocks) {
    // Create a fresh shell session for each TEST block
    const session = new ShellSession(shellConfig);

    try {
      await session.start();

      const result = await executeTestBlock(
        block,
        session,
        filename,
        reporter,
        cwd
      );

      totalAssertionsPassed += result.assertionsPassed;

      if (result.failure) {
        failure = result.failure;
        break;
      }
    } catch (e) {
      failure = {
        filename,
        line: block.testNode?.line ?? 0,
        testName: block.testName,
        error: (e as Error).message,
      };
      break;
    } finally {
      await session.stop();
    }
  }

  console.log(); // Newline after progress dots

  if (failure) {
    return {
      filename,
      passed: false,
      assertionsPassed: totalAssertionsPassed,
      assertionsFailed: 1,
      failure,
    };
  }

  console.log(
    `PASS ${basename(filename)} (${totalAssertionsPassed} assertion${totalAssertionsPassed !== 1 ? "s" : ""})`
  );

  return {
    filename,
    passed: true,
    assertionsPassed: totalAssertionsPassed,
    assertionsFailed: 0,
  };
}

/**
 * A test block with its nodes
 */
interface TestBlock {
  testName?: string;
  testNode?: ASTNode;
  nodes: ASTNode[];
}

/**
 * Group nodes by TEST block
 */
function groupNodesByTest(nodes: ASTNode[]): TestBlock[] {
  const blocks: TestBlock[] = [];
  let currentBlock: TestBlock = { nodes: [] };

  for (const node of nodes) {
    if (node.type === "test") {
      // Start a new block
      if (currentBlock.nodes.length > 0 || currentBlock.testName) {
        blocks.push(currentBlock);
      }
      currentBlock = {
        testName: node.name,
        testNode: node,
        nodes: [],
      };
    } else if (node.type === "pragma" || node.type === "comment") {
      // Skip pragmas and comments - they're already processed
    } else {
      currentBlock.nodes.push(node);
    }
  }

  // Push the last block
  if (currentBlock.nodes.length > 0 || currentBlock.testName) {
    blocks.push(currentBlock);
  }

  return blocks;
}

/**
 * Execute result
 */
interface ExecuteResult {
  assertionsPassed: number;
  failure?: TestFailure;
}

/**
 * Execute a single test block
 */
async function executeTestBlock(
  block: TestBlock,
  session: ShellSession,
  filename: string,
  reporter: Reporter,
  cwd: string
): Promise<ExecuteResult> {
  if (block.testName) {
    session.setCurrentTest(block.testName);
  }

  let lastRunResult: RunResult | undefined;
  const runResults = new Map<string, RunResult>();
  let assertionsPassed = 0;
  const pendingEnvVars: Array<{ key: string; value: string }> = [];

  for (const node of block.nodes) {
    switch (node.type) {
      case "env": {
        pendingEnvVars.push({ key: node.key, value: node.value });
        break;
      }

      case "run": {
        // Apply any pending env vars before the run
        if (pendingEnvVars.length > 0) {
          await session.setEnvVars(pendingEnvVars);
          pendingEnvVars.length = 0;
        }

        try {
          const result = await session.run(node.command, node.name);
          lastRunResult = result;

          if (node.name) {
            runResults.set(node.name, result);
          }

          reporter.onRunComplete(result.runId, true);
        } catch (e) {
          reporter.onRunComplete("", false);
          return {
            assertionsPassed,
            failure: {
              filename,
              line: node.line,
              testName: block.testName,
              runCommand: node.command,
              error: (e as Error).message,
            },
          };
        }
        break;
      }

      case "assert": {
        const result = await evaluateAssertion(
          node,
          lastRunResult,
          runResults,
          session
        );

        if (!result.passed) {
          return {
            assertionsPassed,
            failure: {
              filename,
              line: node.line,
              testName: block.testName,
              runCommand: lastRunResult?.runId,
              assertion: node.raw,
              expected: result.expected,
              actual: result.actual,
              error: result.error,
            },
          };
        }

        assertionsPassed++;
        reporter.onAssertionPass();
        break;
      }
    }
  }

  return { assertionsPassed };
}

/**
 * Evaluate an assertion
 */
async function evaluateAssertion(
  node: AssertNode,
  lastRunResult: RunResult | undefined,
  runResults: Map<string, RunResult>,
  session: ShellSession
): Promise<AssertionResult> {
  const expr = node.expression;

  // Resolve the target run result
  let targetResult: RunResult | undefined;

  if (expr.type !== "file") {
    const target = "target" in expr ? expr.target : undefined;

    if (target) {
      targetResult = runResults.get(target);
      if (!targetResult) {
        return {
          passed: false,
          expected: `RUN named "${target}" to exist`,
          actual: "RUN not found",
          error: `No RUN named "${target}" found`,
        };
      }
    } else {
      targetResult = lastRunResult;
      if (!targetResult) {
        return {
          passed: false,
          expected: "a previous RUN command",
          actual: "no RUN command executed",
          error: "ASSERT without a preceding RUN",
        };
      }
    }
  }

  // Evaluate based on assertion type
  switch (expr.type) {
    case "output": {
      const output = getOutputValue(targetResult!, expr.selector);
      return evaluateOutputPredicate(output, expr.predicate);
    }

    case "exit_code": {
      return evaluateExitCodePredicate(targetResult!.exitCode, expr.predicate);
    }

    case "duration": {
      return evaluateDurationPredicate(targetResult!.durationMs, expr.predicate);
    }

    case "file": {
      // Get current working directory from shell
      const shellCwd = await session.getCwd();
      return evaluateFilePredicate(expr.path, expr.predicate, shellCwd);
    }
  }
}
