/**
 * Output Assertions (stdout/stderr)
 */

import type {
  OutputPredicate,
  RegexLiteral,
  StringLiteral,
} from "../parser/index.ts";

/**
 * Run result type (subset needed for assertions)
 */
export interface RunResult {
  stdout: string;
  stdoutRaw: string;
  stderr: string;
  exitCode: number;
  durationMs: number;
}

/**
 * Assertion result
 */
export interface AssertionResult {
  passed: boolean;
  expected: string;
  actual: string;
  error?: string;
}

/**
 * Get output value based on selector
 */
export function getOutputValue(
  result: RunResult,
  selector: "stdout" | "stdout.raw" | "stderr"
): string {
  switch (selector) {
    case "stdout":
      return result.stdout;
    case "stdout.raw":
      return result.stdoutRaw;
    case "stderr":
      return result.stderr;
  }
}

/**
 * Evaluate an output predicate
 */
export function evaluateOutputPredicate(
  output: string,
  predicate: OutputPredicate
): AssertionResult {
  switch (predicate.type) {
    case "contains":
      return evaluateContains(output, predicate.value);
    case "matches":
      return evaluateMatches(output, predicate.value);
    case "equals":
      return evaluateEquals(output, predicate.operator, predicate.value);
  }
}

/**
 * Evaluate contains predicate
 */
function evaluateContains(
  output: string,
  value: StringLiteral
): AssertionResult {
  const passed = output.includes(value.value);
  return {
    passed,
    expected: `to contain ${value.raw}`,
    actual: output,
  };
}

/**
 * Evaluate matches predicate (regex)
 */
function evaluateMatches(
  output: string,
  value: RegexLiteral
): AssertionResult {
  try {
    const regex = new RegExp(value.pattern, value.flags);
    const passed = regex.test(output);
    return {
      passed,
      expected: `to match ${value.raw}`,
      actual: output,
    };
  } catch (e) {
    return {
      passed: false,
      expected: `to match ${value.raw}`,
      actual: output,
      error: `Invalid regex: ${(e as Error).message}`,
    };
  }
}

/**
 * Evaluate equals predicate (== or !=)
 */
function evaluateEquals(
  output: string,
  operator: "==" | "!=",
  value: StringLiteral
): AssertionResult {
  // Normalize whitespace for comparison
  const normalizedOutput = normalizeWhitespace(output);
  const normalizedValue = normalizeWhitespace(value.value);

  const isEqual = normalizedOutput === normalizedValue;
  const passed = operator === "==" ? isEqual : !isEqual;

  return {
    passed,
    expected: `${operator} ${value.raw}`,
    actual: output,
  };
}

/**
 * Normalize whitespace for comparison
 * - Trim trailing whitespace from each line
 * - Normalize line endings to \n
 */
function normalizeWhitespace(str: string): string {
  return str
    .replace(/\r\n/g, "\n") // Normalize line endings
    .split("\n")
    .map((line) => line.trimEnd()) // Trim trailing whitespace from each line
    .join("\n")
    .trim(); // Trim leading/trailing whitespace from entire string
}
