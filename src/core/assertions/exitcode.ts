import type { ExitCodePredicate } from "../parser/index.ts";
import type { AssertionResult } from "./output.ts";

export function evaluateExitCodePredicate(
  exitCode: number,
  predicate: ExitCodePredicate
): AssertionResult {
  const isEqual = exitCode === predicate.value;
  const passed = predicate.operator === "==" ? isEqual : !isEqual;

  return {
    passed,
    expected: `exit_code ${predicate.operator} ${predicate.value}`,
    actual: String(exitCode),
  };
}
