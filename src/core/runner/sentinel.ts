/**
 * Sentinel Protocol Implementation
 *
 * Handles command framing and completion detection for persistent shell sessions.
 */

// ASCII Unit Separator
const UNIT_SEPARATOR = "\x1f";
const SENTINEL_PREFIX = "__HONE__";

/**
 * Sentinel data parsed from output
 */
export interface SentinelData {
  runId: string;
  exitCode: number;
  endTimestampMs: number;
}

import { basename } from "node:path";

/**
 * Generate a run ID for a command
 */
export function generateRunId(
  filename: string,
  testName: string | undefined,
  runName: string | undefined,
  runIndex: number
): string {
  const parts = [basename(filename).replace(/\.hone$/, "")];

  if (testName) {
    // Sanitize test name for use in ID
    parts.push(testName.replace(/\s+/g, "-").toLowerCase());
  }

  if (runName) {
    parts.push(runName);
  } else {
    parts.push(String(runIndex));
  }

  return parts.join("-");
}

/**
 * Generate the shell wrapper command for a RUN
 */
export function generateShellWrapper(
  command: string,
  runId: string,
  stderrPath: string
): string {
  // Escape single quotes in paths for shell safety
  const escapedStderrPath = stderrPath.replace(/'/g, "'\"'\"'");

  // Shell wrapper uses command grouping {...} to preserve shell state
  // (working directory, variables, etc.) across commands.
  // Note: Commands that would exit the shell (like bare `exit`) should
  // be wrapped in a subshell by the test: (exit 42) instead of exit 42
  return [
    `: > '${escapedStderrPath}'`,
    `{ ${command} ; } 2> '${escapedStderrPath}'`,
    `HONE_EC=$?`,
    `printf "${SENTINEL_PREFIX}${UNIT_SEPARATOR}${runId}${UNIT_SEPARATOR}%d${UNIT_SEPARATOR}%s\\n" "$HONE_EC" "$(date +%s%3N)"`,
  ].join("\n");
}

/**
 * Parse a sentinel line from output
 */
export function parseSentinel(line: string): SentinelData | null {
  // Expected format: __HONE__<US><RUN_ID><US><EXIT_CODE><US><END_TS_MS>
  if (!line.startsWith(SENTINEL_PREFIX)) {
    return null;
  }

  const parts = line.split(UNIT_SEPARATOR);

  if (parts.length !== 4) {
    return null;
  }

  const [, runId, exitCodeStr, timestampStr] = parts;

  if (!runId || !exitCodeStr || !timestampStr) {
    return null;
  }

  const exitCode = parseInt(exitCodeStr, 10);
  const endTimestampMs = parseInt(timestampStr, 10);

  if (isNaN(exitCode) || isNaN(endTimestampMs)) {
    return null;
  }

  return {
    runId,
    exitCode,
    endTimestampMs,
  };
}

/**
 * Check if a line contains a sentinel
 */
export function containsSentinel(line: string): boolean {
  return line.includes(SENTINEL_PREFIX);
}

/**
 * Extract sentinel from output buffer, returning output before sentinel
 * and the sentinel data
 */
export function extractSentinel(
  buffer: string,
  expectedRunId: string
): {
  found: boolean;
  output: string;
  sentinel?: SentinelData;
  remaining: string;
} {
  // Find the sentinel in the buffer - it might be on the same line as output
  // if the command didn't output a trailing newline (e.g., printf)
  const sentinelIndex = buffer.indexOf(SENTINEL_PREFIX);
  
  if (sentinelIndex === -1) {
    return {
      found: false,
      output: buffer,
      remaining: "",
    };
  }

  // Extract everything before the sentinel as output
  const output = buffer.substring(0, sentinelIndex);
  
  // Find the end of the sentinel line (next newline after sentinel)
  const afterSentinel = buffer.substring(sentinelIndex);
  const newlineIndex = afterSentinel.indexOf("\n");
  
  let sentinelLine: string;
  let remaining: string;
  
  if (newlineIndex === -1) {
    // Sentinel line is incomplete (no newline yet)
    return {
      found: false,
      output: buffer,
      remaining: "",
    };
  } else {
    sentinelLine = afterSentinel.substring(0, newlineIndex);
    remaining = afterSentinel.substring(newlineIndex + 1);
  }

  // Parse the sentinel
  const parsed = parseSentinel(sentinelLine.trim());

  if (!parsed || parsed.runId !== expectedRunId) {
    return {
      found: false,
      output: buffer,
      remaining: "",
    };
  }

  // Remove trailing newline from output if present
  const cleanOutput = output.endsWith("\n") 
    ? output.substring(0, output.length - 1) 
    : output;

  return {
    found: true,
    output: cleanOutput,
    sentinel: parsed,
    remaining,
  };
}
