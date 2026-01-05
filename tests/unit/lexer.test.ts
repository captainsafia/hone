import { describe, test, expect } from "bun:test";
import {
  parseStringLiteral,
  parseRegexLiteral,
  parseDuration,
  parseNumber,
  classifyLine,
} from "../../src/core/parser/lexer.ts";

describe("Lexer", () => {
  describe("classifyLine", () => {
    test("classifies empty line", () => {
      expect(classifyLine("", 1).type).toBe("EMPTY");
      expect(classifyLine("   ", 1).type).toBe("EMPTY");
    });

    test("classifies comment", () => {
      expect(classifyLine("# comment", 1).type).toBe("COMMENT");
    });

    test("classifies pragma", () => {
      expect(classifyLine("#! shell: /bin/bash", 1).type).toBe("PRAGMA");
    });

    test("classifies TEST", () => {
      expect(classifyLine('TEST "name"', 1).type).toBe("TEST");
    });

    test("classifies RUN", () => {
      expect(classifyLine("RUN echo hello", 1).type).toBe("RUN");
    });

    test("classifies ASSERT", () => {
      expect(classifyLine("ASSERT exit_code == 0", 1).type).toBe("ASSERT");
    });

    test("classifies ENV", () => {
      expect(classifyLine("ENV FOO=bar", 1).type).toBe("ENV");
    });

    test("classifies unknown", () => {
      expect(classifyLine("UNKNOWN statement", 1).type).toBe("UNKNOWN");
    });
  });

  describe("parseStringLiteral", () => {
    test("parses double-quoted string", () => {
      const result = parseStringLiteral('"hello world"', 0);
      expect(result).not.toBeNull();
      expect(result!.literal.value).toBe("hello world");
      expect(result!.literal.quoteType).toBe("double");
    });

    test("parses single-quoted string", () => {
      const result = parseStringLiteral("'hello world'", 0);
      expect(result).not.toBeNull();
      expect(result!.literal.value).toBe("hello world");
      expect(result!.literal.quoteType).toBe("single");
    });

    test("handles escape sequences in double quotes", () => {
      const result = parseStringLiteral('"line1\\nline2"', 0);
      expect(result).not.toBeNull();
      expect(result!.literal.value).toBe("line1\nline2");
    });

    test("does not escape in single quotes", () => {
      const result = parseStringLiteral("'line1\\nline2'", 0);
      expect(result).not.toBeNull();
      expect(result!.literal.value).toBe("line1\\nline2");
    });

    test("returns null for non-string", () => {
      expect(parseStringLiteral("hello", 0)).toBeNull();
    });

    test("returns null for unterminated string", () => {
      expect(parseStringLiteral('"hello', 0)).toBeNull();
    });

    test("parses string at offset", () => {
      const result = parseStringLiteral('contains "text"', 9);
      expect(result).not.toBeNull();
      expect(result!.literal.value).toBe("text");
    });
  });

  describe("parseRegexLiteral", () => {
    test("parses simple regex", () => {
      const result = parseRegexLiteral("/pattern/", 0);
      expect(result).not.toBeNull();
      expect(result!.literal.pattern).toBe("pattern");
      expect(result!.literal.flags).toBe("");
    });

    test("parses regex with flags", () => {
      const result = parseRegexLiteral("/pattern/gi", 0);
      expect(result).not.toBeNull();
      expect(result!.literal.pattern).toBe("pattern");
      expect(result!.literal.flags).toBe("gi");
    });

    test("handles escaped slashes", () => {
      const result = parseRegexLiteral("/path\\/to\\/file/", 0);
      expect(result).not.toBeNull();
      expect(result!.literal.pattern).toBe("path\\/to\\/file");
    });

    test("returns null for non-regex", () => {
      expect(parseRegexLiteral("pattern", 0)).toBeNull();
    });

    test("returns null for unterminated regex", () => {
      expect(parseRegexLiteral("/pattern", 0)).toBeNull();
    });
  });

  describe("parseDuration", () => {
    test("parses milliseconds", () => {
      const result = parseDuration("200ms", 0);
      expect(result).not.toBeNull();
      expect(result!.duration.value).toBe(200);
      expect(result!.duration.unit).toBe("ms");
    });

    test("parses seconds", () => {
      const result = parseDuration("5s", 0);
      expect(result).not.toBeNull();
      expect(result!.duration.value).toBe(5);
      expect(result!.duration.unit).toBe("s");
    });

    test("parses decimal seconds", () => {
      const result = parseDuration("1.5s", 0);
      expect(result).not.toBeNull();
      expect(result!.duration.value).toBe(1.5);
      expect(result!.duration.unit).toBe("s");
    });

    test("returns null for invalid unit", () => {
      expect(parseDuration("100min", 0)).toBeNull();
    });

    test("returns null for missing unit", () => {
      expect(parseDuration("100", 0)).toBeNull();
    });
  });

  describe("parseNumber", () => {
    test("parses positive integer", () => {
      const result = parseNumber("42", 0);
      expect(result).not.toBeNull();
      expect(result!.value).toBe(42);
    });

    test("parses negative integer", () => {
      const result = parseNumber("-1", 0);
      expect(result).not.toBeNull();
      expect(result!.value).toBe(-1);
    });

    test("parses zero", () => {
      const result = parseNumber("0", 0);
      expect(result).not.toBeNull();
      expect(result!.value).toBe(0);
    });

    test("returns null for non-number", () => {
      expect(parseNumber("abc", 0)).toBeNull();
    });

    test("skips leading whitespace", () => {
      const result = parseNumber("  42", 0);
      expect(result).not.toBeNull();
      expect(result!.value).toBe(42);
    });
  });
});
