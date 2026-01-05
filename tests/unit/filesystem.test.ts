import { describe, test, expect, beforeEach, afterEach } from "bun:test";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { evaluateFilePredicate } from "../../src/core/assertions/filesystem.ts";
import type { StringLiteral, FilePredicate } from "../../src/core/parser/ast.ts";

describe("Filesystem Assertions", () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `hone-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  function makeStringLiteral(value: string): StringLiteral {
    return {
      value,
      raw: `"${value}"`,
      quoteType: "double",
    };
  }

  describe("exists predicate", () => {
    test("passes when file exists", async () => {
      await writeFile(join(testDir, "test.txt"), "content");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        { type: "exists" },
        testDir
      );
      expect(result.passed).toBe(true);
    });

    test("fails when file does not exist", async () => {
      const result = await evaluateFilePredicate(
        makeStringLiteral("nonexistent.txt"),
        { type: "exists" },
        testDir
      );
      expect(result.passed).toBe(false);
    });
  });

  describe("contains predicate", () => {
    test("passes when file contains string", async () => {
      await writeFile(join(testDir, "test.txt"), "hello world");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        { type: "contains", value: makeStringLiteral("world") },
        testDir
      );
      expect(result.passed).toBe(true);
    });

    test("fails when file does not contain string", async () => {
      await writeFile(join(testDir, "test.txt"), "hello world");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        { type: "contains", value: makeStringLiteral("foo") },
        testDir
      );
      expect(result.passed).toBe(false);
    });
  });

  describe("matches predicate", () => {
    test("passes when file matches regex", async () => {
      await writeFile(join(testDir, "test.txt"), "error: line 42");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        {
          type: "matches",
          value: { pattern: "line \\d+", flags: "", raw: "/line \\d+/" },
        },
        testDir
      );
      expect(result.passed).toBe(true);
    });

    test("fails when file does not match regex", async () => {
      await writeFile(join(testDir, "test.txt"), "warning: something");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        {
          type: "matches",
          value: { pattern: "^error", flags: "", raw: "/^error/" },
        },
        testDir
      );
      expect(result.passed).toBe(false);
    });
  });

  describe("equals predicate", () => {
    test("passes on exact content match", async () => {
      await writeFile(join(testDir, "test.txt"), "exact content");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        {
          type: "equals",
          operator: "==",
          value: makeStringLiteral("exact content"),
        },
        testDir
      );
      expect(result.passed).toBe(true);
    });

    test("normalizes trailing whitespace", async () => {
      await writeFile(join(testDir, "test.txt"), "line1  \nline2  \n");
      const result = await evaluateFilePredicate(
        makeStringLiteral("test.txt"),
        {
          type: "equals",
          operator: "==",
          value: makeStringLiteral("line1\nline2"),
        },
        testDir
      );
      expect(result.passed).toBe(true);
    });

    test("handles empty files", async () => {
      await writeFile(join(testDir, "empty.txt"), "");
      const result = await evaluateFilePredicate(
        makeStringLiteral("empty.txt"),
        { type: "equals", operator: "==", value: makeStringLiteral("") },
        testDir
      );
      expect(result.passed).toBe(true);
    });
  });
});
