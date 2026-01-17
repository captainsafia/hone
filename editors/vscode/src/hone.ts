import * as vscode from 'vscode';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export interface HoneTestResult {
  file: string;
  shell: string;
  tests: HoneTest[];
  summary: {
    total_tests: number;
    passed: number;
    failed: number;
    duration_ms: number;
  };
}

export interface HoneTest {
  name: string;
  line: number;
  status: 'passed' | 'failed';
  duration_ms: number;
  runs: HoneRun[];
}

export interface HoneRun {
  name: string | null;
  command: string;
  line: number;
  status: 'passed' | 'failed';
  duration_ms: number;
  exit_code: number;
  stdout: string;
  stderr: string;
  assertions: HoneAssertion[];
}

export interface HoneAssertion {
  line: number;
  expression: string;
  status: 'passed' | 'failed';
  expected?: string;
  actual?: string;
}

/**
 * Find the hone binary in PATH
 */
export async function findHoneBinary(): Promise<string | null> {
  try {
    const { stdout } = await execAsync('which hone');
    const path = stdout.trim();
    if (path) {
      return path;
    }
  } catch (error) {
    // hone not found in PATH
  }
  return null;
}

/**
 * Prompt user to install hone if not found
 */
export async function promptInstallHone(): Promise<boolean> {
  const result = await vscode.window.showErrorMessage(
    'Hone CLI not found in PATH. Would you like to install it?',
    'Install',
    'Manual Instructions',
    'Cancel'
  );

  if (result === 'Install') {
    try {
      const terminal = vscode.window.createTerminal('Install Hone');
      terminal.sendText('curl https://i.safia.sh/captainsafia/hone | sh');
      terminal.show();
      
      vscode.window.showInformationMessage(
        'Hone installation started. Please wait for it to complete and reload VS Code.'
      );
      return true;
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to start installation: ${error}`);
      return false;
    }
  } else if (result === 'Manual Instructions') {
    vscode.env.openExternal(
      vscode.Uri.parse('https://github.com/captainsafia/hone#installation')
    );
  }
  
  return false;
}

/**
 * Run hone command with given arguments
 */
export async function runHoneCommand(
  filePath: string,
  testName?: string,
  useJson = true
): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const honePath = await findHoneBinary();
  
  if (!honePath) {
    throw new Error('Hone binary not found');
  }

  let command = `"${honePath}" run "${filePath}"`;
  
  if (testName) {
    command += ` --test "${testName}"`;
  }
  
  if (useJson) {
    command += ' --output-format json';
  }

  try {
    const { stdout, stderr } = await execAsync(command, {
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer
    });
    return { stdout, stderr, exitCode: 0 };
  } catch (error: any) {
    // Command failed, but we still want the output
    return {
      stdout: error.stdout || '',
      stderr: error.stderr || '',
      exitCode: error.code || 1,
    };
  }
}

/**
 * Parse JSON output from hone
 */
export function parseHoneJsonOutput(output: string): HoneTestResult | null {
  try {
    return JSON.parse(output) as HoneTestResult;
  } catch (error) {
    console.error('Failed to parse hone JSON output:', error);
    return null;
  }
}
