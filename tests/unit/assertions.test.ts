import { describe, test, expect } from "bun:test";
import {
  evaluateOutputPredicate,
  type AssertionResult,
} from "../../src/core/assertions/output.ts";
import { evaluateExitCodePredicate } from "../../src/core/assertions/exitcode.ts";
import { evaluateDurationPredicate } from "../../src/core/assertions/timing.ts";

describe("Assertions", () => {
  describe("Output Assertions", () => {
    describe("contains", () => {
      test("passes when substring exists", () => {
        const result = evaluateOutputPredicate("hello world", {
          type: "contains",
          value: { value: "world", raw: '"world"', quoteType: "double" },
        });
        expect(result.passed).toBe(true);
      });

      test("fails when substring does not exist", () => {
        const result = evaluateOutputPredicate("hello world", {
          type: "contains",
          value: { value: "foo", raw: '"foo"', quoteType: "double" },
        });
        expect(result.passed).toBe(false);
      });

      test("is case-sensitive", () => {
        const result = evaluateOutputPredicate("Hello World", {
          type: "contains",
          value: { value: "hello", raw: '"hello"', quoteType: "double" },
        });
        expect(result.passed).toBe(false);
      });
    });

    describe("matches", () => {
      test("passes when regex matches", () => {
        const result = evaluateOutputPredicate("error: line 42", {
          type: "matches",
          value: { pattern: "error.*\\d+", flags: "", raw: "/error.*\\d+/" },
        });
        expect(result.passed).toBe(true);
      });

      test("fails when regex does not match", () => {
        const result = evaluateOutputPredicate("warning: line 42", {
          type: "matches",
          value: { pattern: "^error", flags: "", raw: "/^error/" },
        });
        expect(result.passed).toBe(false);
      });

      test("respects case-insensitive flag", () => {
        const result = evaluateOutputPredicate("ERROR: something", {
          type: "matches",
          value: { pattern: "error", flags: "i", raw: "/error/i" },
        });
        expect(result.passed).toBe(true);
      });

      test("returns error for invalid regex", () => {
        const result = evaluateOutputPredicate("test", {
          type: "matches",
          value: { pattern: "[invalid", flags: "", raw: "/[invalid/" },
        });
        expect(result.passed).toBe(false);
        expect(result.error).toContain("Invalid regex");
      });
    });

    describe("equals", () => {
      test("passes on exact match with ==", () => {
        const result = evaluateOutputPredicate("hello", {
          type: "equals",
          operator: "==",
          value: { value: "hello", raw: '"hello"', quoteType: "double" },
        });
        expect(result.passed).toBe(true);
      });

      test("fails on mismatch with ==", () => {
        const result = evaluateOutputPredicate("hello", {
          type: "equals",
          operator: "==",
          value: { value: "world", raw: '"world"', quoteType: "double" },
        });
        expect(result.passed).toBe(false);
      });

      test("passes on mismatch with !=", () => {
        const result = evaluateOutputPredicate("hello", {
          type: "equals",
          operator: "!=",
          value: { value: "world", raw: '"world"', quoteType: "double" },
        });
        expect(result.passed).toBe(true);
      });

      test("normalizes trailing whitespace", () => {
        const result = evaluateOutputPredicate("hello  \n  world  ", {
          type: "equals",
          operator: "==",
          value: { value: "hello\n  world", raw: '"hello\\n  world"', quoteType: "double" },
        });
        expect(result.passed).toBe(true);
      });

      test("normalizes line endings", () => {
        const result = evaluateOutputPredicate("hello\r\nworld", {
          type: "equals",
          operator: "==",
          value: { value: "hello\nworld", raw: '"hello\\nworld"', quoteType: "double" },
        });
        expect(result.passed).toBe(true);
      });
    });
  });

  describe("Exit Code Assertions", () => {
    test("passes when exit code matches with ==", () => {
      const result = evaluateExitCodePredicate(0, { operator: "==", value: 0 });
      expect(result.passed).toBe(true);
    });

    test("fails when exit code does not match with ==", () => {
      const result = evaluateExitCodePredicate(1, { operator: "==", value: 0 });
      expect(result.passed).toBe(false);
    });

    test("passes when exit code does not match with !=", () => {
      const result = evaluateExitCodePredicate(127, { operator: "!=", value: 0 });
      expect(result.passed).toBe(true);
    });

    test("fails when exit code matches with !=", () => {
      const result = evaluateExitCodePredicate(0, { operator: "!=", value: 0 });
      expect(result.passed).toBe(false);
    });
  });

  describe("Duration Assertions", () => {
    test("passes when duration < threshold", () => {
      const result = evaluateDurationPredicate(100, {
        operator: "<",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });

    test("fails when duration >= threshold with <", () => {
      const result = evaluateDurationPredicate(200, {
        operator: "<",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(false);
    });

    test("passes when duration <= threshold", () => {
      const result = evaluateDurationPredicate(200, {
        operator: "<=",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });

    test("converts seconds to milliseconds", () => {
      const result = evaluateDurationPredicate(1500, {
        operator: "<=",
        value: { value: 1.5, unit: "s", raw: "1.5s" },
      });
      expect(result.passed).toBe(true);
    });

    test("handles > operator", () => {
      const result = evaluateDurationPredicate(300, {
        operator: ">",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });

    test("handles >= operator", () => {
      const result = evaluateDurationPredicate(200, {
        operator: ">=",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });

    test("handles == operator", () => {
      const result = evaluateDurationPredicate(200, {
        operator: "==",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });

    test("handles != operator", () => {
      const result = evaluateDurationPredicate(300, {
        operator: "!=",
        value: { value: 200, unit: "ms", raw: "200ms" },
      });
      expect(result.passed).toBe(true);
    });
  });
});
