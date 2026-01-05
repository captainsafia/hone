import { describe, test, expect } from "bun:test";
import {
  generateRunId,
  generateShellWrapper,
  parseSentinel,
  extractSentinel,
} from "../../src/core/runner/sentinel.ts";

describe("Sentinel Protocol", () => {
  describe("generateRunId", () => {
    test("generates ID with test name and run name", () => {
      const id = generateRunId("test.hone", "my test", "build", 1);
      expect(id).toBe("test-my-test-build");
    });

    test("generates ID with test name and run index", () => {
      const id = generateRunId("test.hone", "my test", undefined, 3);
      expect(id).toBe("test-my-test-3");
    });

    test("generates ID without test name", () => {
      const id = generateRunId("test.hone", undefined, "build", 1);
      expect(id).toBe("test-build");
    });

    test("strips .hone extension", () => {
      const id = generateRunId("example.hone", undefined, undefined, 1);
      expect(id).toBe("example-1");
    });
  });

  describe("generateShellWrapper", () => {
    test("generates wrapper with command", () => {
      const wrapper = generateShellWrapper(
        "echo hello",
        "test-1",
        "/tmp/stderr.txt"
      );

      expect(wrapper).toContain("echo hello");
      expect(wrapper).toContain("__HONE__");
      expect(wrapper).toContain("test-1");
      expect(wrapper).toContain("/tmp/stderr.txt");
      expect(wrapper).toContain("HONE_EC=$?");
    });

    test("escapes single quotes in stderr path", () => {
      const wrapper = generateShellWrapper(
        "echo hello",
        "test-1",
        "/tmp/it's a path/stderr.txt"
      );

      // Should escape single quote properly
      expect(wrapper).toContain("'\"'\"'");
    });
  });

  describe("parseSentinel", () => {
    test("parses valid sentinel", () => {
      const sentinel = "__HONE__\x1ftest-1\x1f0\x1f1704326400000";
      const result = parseSentinel(sentinel);

      expect(result).not.toBeNull();
      expect(result!.runId).toBe("test-1");
      expect(result!.exitCode).toBe(0);
      expect(result!.endTimestampMs).toBe(1704326400000);
    });

    test("parses non-zero exit code", () => {
      const sentinel = "__HONE__\x1ftest-1\x1f127\x1f1704326400000";
      const result = parseSentinel(sentinel);

      expect(result).not.toBeNull();
      expect(result!.exitCode).toBe(127);
    });

    test("returns null for non-sentinel", () => {
      expect(parseSentinel("echo hello")).toBeNull();
    });

    test("returns null for malformed sentinel", () => {
      expect(parseSentinel("__HONE__\x1ftest-1")).toBeNull();
      expect(parseSentinel("__HONE__\x1ftest-1\x1fabc\x1f123")).toBeNull();
    });
  });

  describe("extractSentinel", () => {
    test("extracts sentinel from output", () => {
      const buffer =
        "hello world\nsome output\n__HONE__\x1ftest-1\x1f0\x1f1704326400000\n";
      const result = extractSentinel(buffer, "test-1");

      expect(result.found).toBe(true);
      expect(result.output).toBe("hello world\nsome output");
      expect(result.sentinel).not.toBeUndefined();
      expect(result.sentinel!.exitCode).toBe(0);
    });

    test("returns remaining content after sentinel", () => {
      const buffer =
        "output\n__HONE__\x1ftest-1\x1f0\x1f1704326400000\nextra content";
      const result = extractSentinel(buffer, "test-1");

      expect(result.found).toBe(true);
      expect(result.remaining).toBe("extra content");
    });

    test("does not match wrong run ID", () => {
      const buffer = "output\n__HONE__\x1ftest-1\x1f0\x1f1704326400000\n";
      const result = extractSentinel(buffer, "test-2");

      expect(result.found).toBe(false);
    });

    test("returns all content if no sentinel", () => {
      const buffer = "hello world\nsome output\n";
      const result = extractSentinel(buffer, "test-1");

      expect(result.found).toBe(false);
      expect(result.output).toBe("hello world\nsome output\n");
    });
  });
});
