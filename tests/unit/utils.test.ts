import { describe, test, expect } from "bun:test";
import { stripAnsiCodes, hasAnsiCodes } from "../../src/core/utils/ansi.ts";
import {
  validatePathSecurity,
  containsDangerousPatterns,
} from "../../src/core/utils/security.ts";

describe("ANSI Utilities", () => {
  test("strips ANSI codes", () => {
    const input = "\x1b[32mgreen text\x1b[0m";
    expect(stripAnsiCodes(input)).toBe("green text");
  });

  test("returns plain text unchanged", () => {
    const input = "plain text";
    expect(stripAnsiCodes(input)).toBe("plain text");
  });

  test("detects ANSI codes", () => {
    expect(hasAnsiCodes("\x1b[32mtext\x1b[0m")).toBe(true);
    expect(hasAnsiCodes("plain text")).toBe(false);
  });
});

describe("Security Utilities", () => {
  describe("validatePathSecurity", () => {
    test("allows paths within base directory", () => {
      const result = validatePathSecurity("subdir/file.txt", "/home/user/project");
      expect(result.valid).toBe(true);
    });

    test("rejects path traversal with ..", () => {
      const result = validatePathSecurity("../outside/file.txt", "/home/user/project");
      expect(result.valid).toBe(false);
    });

    test("allows absolute paths within base", () => {
      const result = validatePathSecurity(
        "/home/user/project/file.txt",
        "/home/user/project"
      );
      expect(result.valid).toBe(true);
    });

    test("rejects absolute paths outside base", () => {
      const result = validatePathSecurity("/etc/passwd", "/home/user/project");
      expect(result.valid).toBe(false);
    });
  });

  describe("containsDangerousPatterns", () => {
    test("detects parent directory traversal", () => {
      expect(containsDangerousPatterns("../file")).toBe(true);
      expect(containsDangerousPatterns("path/../file")).toBe(true);
    });

    test("detects system paths", () => {
      expect(containsDangerousPatterns("/etc/passwd")).toBe(true);
      expect(containsDangerousPatterns("/var/log/syslog")).toBe(true);
      expect(containsDangerousPatterns("/proc/self/environ")).toBe(true);
    });

    test("allows safe paths", () => {
      expect(containsDangerousPatterns("file.txt")).toBe(false);
      expect(containsDangerousPatterns("subdir/file.txt")).toBe(false);
    });
  });
});
