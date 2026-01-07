import { spawn, type Subprocess } from "bun";
import { mkdir, readFile } from "node:fs/promises";
import { join, basename } from "node:path";
import { stripAnsiCodes } from "../utils/ansi.ts";
import {
  generateRunId,
  generateShellWrapper,
  extractSentinel,
  type SentinelData,
} from "./sentinel.ts";
import type { PragmaNode } from "../parser/index.ts";

export interface ShellConfig {
  shell: string;
  env: Record<string, string>;
  timeout: number; // milliseconds
  cwd: string;
  filename: string;
}

export interface RunResult {
  runId: string;
  stdout: string;
  stdoutRaw: string;
  stderr: string;
  exitCode: number;
  durationMs: number;
  stderrPath: string;
}

const SHELL_FLAGS: Record<string, string[]> = {
  bash: ["--norc", "--noprofile"],
  zsh: ["--no-rcs"],
  sh: [],
};

function getShellFlags(shellPath: string): string[] {
  const shellName = basename(shellPath);
  return SHELL_FLAGS[shellName] ?? [];
}

export function isShellSupported(shellPath: string): boolean {
  const shellName = basename(shellPath);
  return shellName in SHELL_FLAGS;
}

export class ShellSession {
  private process: Subprocess<"pipe", "pipe", "pipe"> | null = null;
  private outputBuffer: string = "";
  private outputPromise: Promise<void> | null = null;
  private reading: boolean = false;
  private config: ShellConfig;
  private runIndex: number = 0;
  private currentTestName: string | undefined;
  private artifactDir: string;

  constructor(config: ShellConfig) {
    this.config = config;
    this.artifactDir = join(
      config.cwd,
      ".hone",
      "runs",
      `${this.getTimestamp()}-${basename(config.filename, ".hone")}`
    );
  }

  private getTimestamp(): string {
    const now = new Date();
    return now.toISOString().replace(/[:.]/g, "-").replace("T", "_").substring(0, 19);
  }

  async start(): Promise<void> {
    const shellFlags = getShellFlags(this.config.shell);

    // Create artifact directory
    await mkdir(this.artifactDir, { recursive: true });

    // Build environment
    const env: Record<string, string> = {
      ...this.config.env,
      PS1: "", // Suppress prompt
      TERM: "dumb", // Simple terminal
    };

    // Start shell process
    this.process = spawn([this.config.shell, ...shellFlags], {
      cwd: this.config.cwd,
      env,
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });

    // Start continuous stdout reading
    this.startOutputReader();

    // Wait for shell to be ready
    await this.waitForReady();
  }

  private startOutputReader(): void {
    if (!this.process?.stdout || this.reading) {
      return;
    }

    this.reading = true;
    const reader = this.process.stdout.getReader();
    const decoder = new TextDecoder();

    this.outputPromise = (async () => {
      try {
        while (this.reading) {
          const { done, value } = await reader.read();
          if (done) break;
          if (value) {
            this.outputBuffer += decoder.decode(value);
          }
        }
      } catch {
        // Reader closed or process ended
      } finally {
        try {
          reader.releaseLock();
        } catch {
          // Ignore
        }
      }
    })();
  }

  private async waitForString(marker: string, timeoutMs: number): Promise<boolean> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      if (this.outputBuffer.includes(marker)) {
        return true;
      }
      await new Promise((resolve) => setTimeout(resolve, 10));
    }

    return false;
  }

  private clearBuffer(): void {
    this.outputBuffer = "";
  }

  private async waitForReady(): Promise<void> {
    // Send a simple echo command and wait for response
    const readyMarker = `__HONE_READY_${Date.now()}__`;
    await this.writeToShell(`echo "${readyMarker}"\n`);

    const found = await this.waitForString(readyMarker, 5000);

    if (!found) {
      throw new Error(
        `Shell failed to start within 5000ms. Shell: ${this.config.shell}`
      );
    }

    // Clear the buffer of startup noise
    this.clearBuffer();
  }

  setCurrentTest(testName: string | undefined): void {
    this.currentTestName = testName;
  }

  async setEnvVars(vars: Array<{ key: string; value: string }>): Promise<void> {
    for (const { key, value } of vars) {
      await this.writeToShell(`export ${key}='${value.replace(/'/g, "'\\''")}'\n`);
    }

    // Wait a bit for the exports to complete
    await this.flush();
  }

  private async flush(): Promise<void> {
    const flushMarker = `__HONE_FLUSH_${Date.now()}__`;
    await this.writeToShell(`echo "${flushMarker}"\n`);

    await this.waitForString(flushMarker, 2000);
    this.clearBuffer();
  }

  async run(command: string, name?: string): Promise<RunResult> {
    if (!this.process) {
      throw new Error("Shell session not started");
    }

    this.runIndex++;
    const runId = generateRunId(
      this.config.filename,
      this.currentTestName,
      name,
      this.runIndex
    );

    // Create stderr file path
    const stderrPath = join(this.artifactDir, `${runId}-stderr.txt`);

    // Generate and send shell wrapper
    const wrapper = generateShellWrapper(command, runId, stderrPath);
    const startTime = Date.now();

    await this.writeToShell(wrapper + "\n");

    // Wait for sentinel
    const result = await this.waitForSentinel(runId);
    const endTime = Date.now();

    // Read stderr from file
    let stderr = "";
    try {
      stderr = await readFile(stderrPath, "utf-8");
    } catch {
      // File might not exist if command didn't produce stderr
    }

    return {
      runId,
      stdout: stripAnsiCodes(result.output),
      stdoutRaw: result.output,
      stderr,
      exitCode: result.sentinel?.exitCode ?? -1,
      durationMs: endTime - startTime,
      stderrPath,
    };
  }

  private async waitForSentinel(
    runId: string
  ): Promise<{ output: string; sentinel?: SentinelData }> {
    const startTime = Date.now();

    while (Date.now() - startTime < this.config.timeout) {
      const result = extractSentinel(this.outputBuffer, runId);

      if (result.found) {
        // Remove the consumed content from buffer
        this.outputBuffer = result.remaining;
        return {
          output: result.output,
          sentinel: result.sentinel,
        };
      }

      await new Promise((resolve) => setTimeout(resolve, 10));
    }

    throw new Error(
      `Timeout waiting for command completion (${this.config.timeout}ms). Run ID: ${runId}`
    );
  }

  async getCwd(): Promise<string> {
    const marker = `__HONE_CWD_${Date.now()}__`;
    await this.writeToShell(`echo "${marker}$PWD${marker}"\n`);

    const found = await this.waitForString(marker, 2000);
    if (!found) {
      return this.config.cwd; // Fallback
    }

    const match = this.outputBuffer.match(new RegExp(`${marker}(.+?)${marker}`));
    if (match) {
      this.clearBuffer();
      return match[1]!;
    }

    return this.config.cwd; // Fallback
  }

  private async writeToShell(data: string): Promise<void> {
    if (!this.process?.stdin) {
      throw new Error("Shell stdin not available");
    }

    this.process.stdin.write(data);
  }

  async stop(): Promise<void> {
    this.reading = false;

    if (this.process) {
      try {
        await this.writeToShell("exit\n");
      } catch {
        // Ignore errors when exiting
      }

      this.process.kill();
      this.process = null;
    }

    // Wait for reader to finish
    if (this.outputPromise) {
      try {
        await Promise.race([
          this.outputPromise,
          new Promise((resolve) => setTimeout(resolve, 100)),
        ]);
      } catch {
        // Ignore
      }
    }
  }

  [Symbol.dispose](): void {
    this.stop().catch(() => {});
  }
}

export function createShellConfig(
  pragmas: PragmaNode[],
  filename: string,
  cwd: string,
  overrideShell?: string
): ShellConfig {
  let shell = overrideShell ?? process.env["SHELL"] ?? "/bin/bash";
  const env: Record<string, string> = {
    PATH: process.env["PATH"] ?? "/usr/bin:/bin",
    HOME: process.env["HOME"] ?? "/",
  };
  let timeout = 30000; // 30 seconds default

  for (const pragma of pragmas) {
    switch (pragma.pragmaType) {
      case "shell":
        if (!overrideShell) {
          shell = pragma.value;
        }
        break;
      case "env":
        if (pragma.key) {
          env[pragma.key] = pragma.value;
        }
        break;
      case "timeout": {
        // Parse timeout value
        const match = pragma.value.match(/^(\d+(?:\.\d+)?)(ms|s)$/);
        if (match) {
          const value = parseFloat(match[1]!);
          const unit = match[2];
          timeout = unit === "s" ? value * 1000 : value;
        }
        break;
      }
    }
  }

  return {
    shell,
    env,
    timeout,
    cwd,
    filename,
  };
}
