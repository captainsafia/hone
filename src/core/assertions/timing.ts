/**
 * Timing/Duration Assertions
 */

import type { DurationPredicate, Duration, ComparisonOperator } from "../parser/index.ts";
import type { AssertionResult } from "./output.ts";

/**
 * Convert duration to milliseconds
 */
export function durationToMs(duration: Duration): number {
  return duration.unit === "s" ? duration.value * 1000 : duration.value;
}

/**
 * Format duration for display
 */
export function formatDuration(ms: number): string {
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(2)}s`;
  }
  return `${ms}ms`;
}

/**
 * Evaluate duration predicate
 */
export function evaluateDurationPredicate(
  durationMs: number,
  predicate: DurationPredicate
): AssertionResult {
  const expectedMs = durationToMs(predicate.value);
  const passed = evaluateComparison(durationMs, predicate.operator, expectedMs);

  return {
    passed,
    expected: `duration ${predicate.operator} ${predicate.value.raw}`,
    actual: formatDuration(durationMs),
  };
}

/**
 * Evaluate a numeric comparison
 */
function evaluateComparison(
  actual: number,
  operator: ComparisonOperator,
  expected: number
): boolean {
  switch (operator) {
    case "==":
      return actual === expected;
    case "!=":
      return actual !== expected;
    case "<":
      return actual < expected;
    case "<=":
      return actual <= expected;
    case ">":
      return actual > expected;
    case ">=":
      return actual >= expected;
  }
}
