/**
 * Reporter Interface and Default Implementation
 *
 * Handles progress output and error reporting.
 */

import chalk from "chalk";
import type { ParseError } from "../parser/index.ts";

/**
 * Test failure information
 */
export interface TestFailure {
  filename: string;
  line: number;
  testName?: string;
  runCommand?: string;
  assertion?: string;
  expected?: string;
  actual?: string;
  error?: string;
}

/**
 * Test results summary
 */
export interface TestResults {
  totalFiles: number;
  passedFiles: number;
  failedFiles: number;
  totalAssertions: number;
  passedAssertions: number;
  failedAssertions: number;
  failures: TestFailure[];
}

/**
 * Reporter interface for progress and error output
 */
export interface Reporter {
  onFileStart(filename: string): void;
  onRunComplete(runId: string, success: boolean): void;
  onTestComplete(testName: string, passed: boolean): void;
  onAssertionPass(): void;
  onFailure(error: TestFailure): void;
  onParseErrors(errors: ParseError[]): void;
  onWarning(message: string): void;
  onSummary(results: TestResults): void;
}

/**
 * Default reporter implementation
 */
export class DefaultReporter implements Reporter {
  private verbose: boolean;
  private currentFile: string = "";
  private runCount: number = 0;

  constructor(options: { verbose?: boolean } = {}) {
    this.verbose = options.verbose ?? false;
  }

  onFileStart(filename: string): void {
    this.currentFile = filename;
    this.runCount = 0;
    console.log(`Running ${filename}`);
  }

  onRunComplete(_runId: string, success: boolean): void {
    this.runCount++;
    process.stdout.write(success ? chalk.green("✓") : chalk.red("✗"));
  }

  onTestComplete(_testName: string, passed: boolean): void {
    if (passed) {
      process.stdout.write(chalk.green("."));
    }
  }

  onAssertionPass(): void {
    // Silent by default
  }

  onFailure(failure: TestFailure): void {
    console.log(); // Newline after progress dots
    console.log();
    console.log(
      chalk.red("FAIL"),
      chalk.dim(`${failure.filename}:${failure.line}`),
      failure.testName ? chalk.dim(`:: "${failure.testName}"`) : ""
    );

    if (failure.runCommand) {
      console.log(chalk.dim("RUN:"), failure.runCommand);
    }

    if (failure.assertion) {
      console.log(chalk.dim("ASSERT:"), failure.assertion);
    }

    if (failure.expected) {
      console.log(chalk.yellow("Expected:"), failure.expected);
    }

    if (failure.actual !== undefined) {
      console.log(chalk.yellow("Actual:"));
      // Indent actual output
      const lines = failure.actual.split("\n");
      for (const line of lines.slice(0, this.verbose ? undefined : 10)) {
        console.log(chalk.dim("  "), line);
      }
      if (!this.verbose && lines.length > 10) {
        console.log(chalk.dim(`  ... (${lines.length - 10} more lines)`));
      }
    }

    if (failure.error) {
      console.log(chalk.red("Error:"), failure.error);
    }
  }

  onParseErrors(errors: ParseError[]): void {
    for (const error of errors) {
      console.log(
        chalk.red("Parse Error:"),
        chalk.dim(`${error.filename}:${error.line}`),
        error.message
      );
    }
  }

  onWarning(message: string): void {
    console.error(chalk.yellow("Warning:"), message);
  }

  onSummary(results: TestResults): void {
    console.log();

    if (results.failedFiles === 0) {
      console.log(
        chalk.green("✓"),
        `All tests passed (${results.totalFiles} file${results.totalFiles !== 1 ? "s" : ""}, ${results.passedAssertions} assertion${results.passedAssertions !== 1 ? "s" : ""})`
      );
    } else {
      console.log(
        chalk.red("✗"),
        `${results.failedFiles} of ${results.totalFiles} file${results.totalFiles !== 1 ? "s" : ""} failed`
      );
      console.log(
        chalk.dim(
          `  ${results.passedAssertions} passed, ${results.failedAssertions} failed`
        )
      );
    }
  }
}
