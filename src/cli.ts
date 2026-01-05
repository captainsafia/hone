#!/usr/bin/env bun

/**
 * hone - CLI Integration Test Runner
 *
 * A testing tool for command-line applications.
 */

import { Command } from "commander";
import chalk from "chalk";
import { runTests, DefaultReporter } from "./core/runner/index.ts";

// Import package.json for version
const packageJson = await import("../package.json");

const program = new Command();

program
  .name("hone")
  .description("CLI integration testing tool")
  .version(packageJson.version);

// Run command (default)
program
  .command("run", { isDefault: true })
  .description("Run test files")
  .argument("<patterns...>", "Test file paths, directories, or glob patterns")
  .option("--shell <path>", "Override shell executable")
  .option("--verbose", "Include full stdout/stderr dumps on failure")
  .action(async (patterns: string[], options: { shell?: string; verbose?: boolean }) => {
    try {
      const results = await runTests(patterns, {
        shell: options.shell,
        verbose: options.verbose,
        reporter: new DefaultReporter({ verbose: options.verbose }),
      });

      // Exit with appropriate code
      process.exit(results.failedFiles > 0 ? 1 : 0);
    } catch (error) {
      console.error(chalk.red("Error:"), (error as Error).message);
      process.exit(1);
    }
  });

// Handle unknown commands
program.on("command:*", () => {
  console.error(chalk.red("Invalid command:"), program.args.join(" "));
  console.log(chalk.yellow("See --help for a list of available commands."));
  process.exit(1);
});

// Global error handlers
process.on("uncaughtException", (error) => {
  console.error(chalk.red("Uncaught Exception:"), error.message);
  process.exit(1);
});

process.on("unhandledRejection", (reason, _promise) => {
  console.error(chalk.red("Unhandled Rejection:"), reason);
  process.exit(1);
});

// Parse and execute
await program.parseAsync();

// Show help if no command provided
if (!process.argv.slice(2).length) {
  program.outputHelp();
}
