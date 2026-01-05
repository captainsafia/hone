import { describe, test, expect } from "bun:test";
import {
  parseFile,
  type ParseResult,
  type PragmaNode,
  type TestNode,
  type RunNode,
  type AssertNode,
  type EnvNode,
} from "../../src/core/parser/index.ts";

describe("Parser", () => {
  describe("Pragmas", () => {
    test("parses shell pragma", () => {
      const result = parseFile("#! shell: /bin/zsh", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      expect(result.file.pragmas).toHaveLength(1);
      const pragma = result.file.pragmas[0] as PragmaNode;
      expect(pragma.pragmaType).toBe("shell");
      expect(pragma.value).toBe("/bin/zsh");
    });

    test("parses env pragma", () => {
      const result = parseFile("#! env: PATH=/custom/bin", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      expect(result.file.pragmas).toHaveLength(1);
      const pragma = result.file.pragmas[0] as PragmaNode;
      expect(pragma.pragmaType).toBe("env");
      expect(pragma.key).toBe("PATH");
      expect(pragma.value).toBe("/custom/bin");
    });

    test("parses timeout pragma with seconds", () => {
      const result = parseFile("#! timeout: 60s", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const pragma = result.file.pragmas[0] as PragmaNode;
      expect(pragma.pragmaType).toBe("timeout");
      expect(pragma.value).toBe("60s");
    });

    test("parses timeout pragma with milliseconds", () => {
      const result = parseFile("#! timeout: 500ms", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const pragma = result.file.pragmas[0] as PragmaNode;
      expect(pragma.pragmaType).toBe("timeout");
      expect(pragma.value).toBe("500ms");
    });

    test("warns on unknown pragma", () => {
      const result = parseFile("#! unknown: value", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      expect(result.file.warnings).toHaveLength(1);
      expect(result.file.warnings[0]?.message).toContain("Unknown pragma");
    });

    test("errors if pragma appears after non-pragma", () => {
      const result = parseFile("RUN echo test\n#! shell: /bin/zsh", "test.hone");
      expect(result.success).toBe(false);
      if (result.success) return;

      expect(result.errors[0]?.message).toContain("top of the file");
    });
  });

  describe("TEST blocks", () => {
    test("parses test with double-quoted name", () => {
      const result = parseFile('TEST "init works"', "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const test = result.file.nodes.find((n) => n.type === "test") as TestNode;
      expect(test.name).toBe("init works");
    });

    test("parses test with single-quoted name", () => {
      const result = parseFile("TEST 'build and deploy'", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const test = result.file.nodes.find((n) => n.type === "test") as TestNode;
      expect(test.name).toBe("build and deploy");
    });

    test("allows alphanumeric, spaces, dashes, underscores in test names", () => {
      const result = parseFile('TEST "test-name_123 with spaces"', "test.hone");
      expect(result.success).toBe(true);
    });

    test("rejects special characters in test names", () => {
      const result = parseFile('TEST "test@name"', "test.hone");
      expect(result.success).toBe(false);
    });
  });

  describe("RUN statements", () => {
    test("parses simple RUN", () => {
      const result = parseFile("RUN echo hello", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const run = result.file.nodes.find((n) => n.type === "run") as RunNode;
      expect(run.command).toBe("echo hello");
      expect(run.name).toBeUndefined();
    });

    test("parses named RUN", () => {
      const result = parseFile("RUN build: npm run build", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const run = result.file.nodes.find((n) => n.type === "run") as RunNode;
      expect(run.name).toBe("build");
      expect(run.command).toBe("npm run build");
    });

    test("errors on duplicate RUN names", () => {
      const result = parseFile(
        "RUN build: npm run build\nRUN build: npm run test",
        "test.hone"
      );
      expect(result.success).toBe(false);
      if (result.success) return;

      expect(result.errors[0]?.message).toContain("Duplicate RUN name");
    });

    test("errors on empty command", () => {
      const result = parseFile("RUN ", "test.hone");
      expect(result.success).toBe(false);
    });
  });

  describe("ENV statements", () => {
    test("parses ENV statement", () => {
      const result = parseFile("ENV FOO=bar", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const env = result.file.nodes.find((n) => n.type === "env") as EnvNode;
      expect(env.key).toBe("FOO");
      expect(env.value).toBe("bar");
    });
  });

  describe("ASSERT statements", () => {
    describe("output assertions", () => {
      test("parses stdout contains", () => {
        const result = parseFile('ASSERT stdout contains "hello"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("output");
        if (assert.expression.type !== "output") return;
        expect(assert.expression.selector).toBe("stdout");
        expect(assert.expression.predicate.type).toBe("contains");
      });

      test("parses stderr matches regex", () => {
        const result = parseFile("ASSERT stderr matches /error/i", "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("output");
        if (assert.expression.type !== "output") return;
        expect(assert.expression.selector).toBe("stderr");
        expect(assert.expression.predicate.type).toBe("matches");
        if (assert.expression.predicate.type !== "matches") return;
        expect(assert.expression.predicate.value.pattern).toBe("error");
        expect(assert.expression.predicate.value.flags).toBe("i");
      });

      test("parses stdout.raw contains", () => {
        const result = parseFile('ASSERT stdout.raw contains "\\x1b[32m"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("output");
        if (assert.expression.type !== "output") return;
        expect(assert.expression.selector).toBe("stdout.raw");
      });

      test("parses stdout == exact", () => {
        const result = parseFile('ASSERT stdout == "exact text"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("output");
        if (assert.expression.type !== "output") return;
        expect(assert.expression.predicate.type).toBe("equals");
        if (assert.expression.predicate.type !== "equals") return;
        expect(assert.expression.predicate.operator).toBe("==");
      });

      test("parses named target assertion", () => {
        const result = parseFile('ASSERT build.stdout contains "success"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("output");
        if (assert.expression.type !== "output") return;
        expect(assert.expression.target).toBe("build");
      });
    });

    describe("exit code assertions", () => {
      test("parses exit_code ==", () => {
        const result = parseFile("ASSERT exit_code == 0", "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("exit_code");
        if (assert.expression.type !== "exit_code") return;
        expect(assert.expression.predicate.operator).toBe("==");
        expect(assert.expression.predicate.value).toBe(0);
      });

      test("parses named exit_code !=", () => {
        const result = parseFile("ASSERT build.exit_code != 127", "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("exit_code");
        if (assert.expression.type !== "exit_code") return;
        expect(assert.expression.target).toBe("build");
        expect(assert.expression.predicate.value).toBe(127);
      });
    });

    describe("duration assertions", () => {
      test("parses duration < ms", () => {
        const result = parseFile("ASSERT duration < 200ms", "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("duration");
        if (assert.expression.type !== "duration") return;
        expect(assert.expression.predicate.operator).toBe("<");
        expect(assert.expression.predicate.value.value).toBe(200);
        expect(assert.expression.predicate.value.unit).toBe("ms");
      });

      test("parses duration <= seconds with decimal", () => {
        const result = parseFile("ASSERT duration <= 1.5s", "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("duration");
        if (assert.expression.type !== "duration") return;
        expect(assert.expression.predicate.value.value).toBe(1.5);
        expect(assert.expression.predicate.value.unit).toBe("s");
      });
    });

    describe("file assertions", () => {
      test("parses file exists", () => {
        const result = parseFile('ASSERT file "out.txt" exists', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("file");
        if (assert.expression.type !== "file") return;
        expect(assert.expression.path.value).toBe("out.txt");
        expect(assert.expression.predicate.type).toBe("exists");
      });

      test("parses file contains", () => {
        const result = parseFile('ASSERT file "out.txt" contains "OK"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("file");
        if (assert.expression.type !== "file") return;
        expect(assert.expression.predicate.type).toBe("contains");
      });

      test("parses file matches regex", () => {
        const result = parseFile('ASSERT file "out.txt" matches /OK:\\s+\\d+/', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("file");
        if (assert.expression.type !== "file") return;
        expect(assert.expression.predicate.type).toBe("matches");
      });

      test("parses file ==", () => {
        const result = parseFile('ASSERT file "out.txt" == "exact contents\\n"', "test.hone");
        expect(result.success).toBe(true);
        if (!result.success) return;

        const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
        expect(assert.expression.type).toBe("file");
        if (assert.expression.type !== "file") return;
        expect(assert.expression.predicate.type).toBe("equals");
      });
    });
  });

  describe("String literals", () => {
    test("single quotes are literal (no escape sequences)", () => {
      const result = parseFile("ASSERT stdout contains 'hello\\nworld'", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
      if (assert.expression.type !== "output") return;
      if (assert.expression.predicate.type !== "contains") return;
      // Literal backslash-n, not newline
      expect(assert.expression.predicate.value.value).toBe("hello\\nworld");
    });

    test("double quotes support escape sequences", () => {
      const result = parseFile('ASSERT stdout contains "hello\\nworld"', "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
      if (assert.expression.type !== "output") return;
      if (assert.expression.predicate.type !== "contains") return;
      // Actual newline
      expect(assert.expression.predicate.value.value).toBe("hello\nworld");
    });

    test("double quotes support tab escape", () => {
      const result = parseFile('ASSERT stdout contains "a\\tb"', "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
      if (assert.expression.type !== "output") return;
      if (assert.expression.predicate.type !== "contains") return;
      expect(assert.expression.predicate.value.value).toBe("a\tb");
    });

    test("double quotes support escaped quote", () => {
      const result = parseFile('ASSERT stdout contains "say \\"hello\\""', "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
      if (assert.expression.type !== "output") return;
      if (assert.expression.predicate.type !== "contains") return;
      expect(assert.expression.predicate.value.value).toBe('say "hello"');
    });

    test("double quotes support escaped backslash", () => {
      const result = parseFile('ASSERT stdout contains "path\\\\file"', "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const assert = result.file.nodes.find((n) => n.type === "assert") as AssertNode;
      if (assert.expression.type !== "output") return;
      if (assert.expression.predicate.type !== "contains") return;
      expect(assert.expression.predicate.value.value).toBe("path\\file");
    });
  });

  describe("Comments", () => {
    test("parses comments", () => {
      const result = parseFile("# This is a comment\nRUN echo test", "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      const comment = result.file.nodes.find((n) => n.type === "comment");
      expect(comment).toBeDefined();
    });

    test("ignores empty lines", () => {
      const result = parseFile("\n\nRUN echo test\n\n", "test.hone");
      expect(result.success).toBe(true);
    });
  });

  describe("Complete test file", () => {
    test("parses a complete test file", () => {
      const content = `
#! shell: /bin/bash
#! timeout: 60s
#! env: DEBUG=true

TEST "initialization test"

ENV PROJECT_NAME=myapp

RUN init: myapp init
ASSERT exit_code == 0
ASSERT stdout contains "initialized"

RUN build: myapp build
ASSERT exit_code == 0
ASSERT duration < 5s
ASSERT file "dist/main.js" exists

TEST "error handling"

RUN myapp invalid-command
ASSERT exit_code != 0
ASSERT stderr contains "unknown command"
`.trim();

      const result = parseFile(content, "test.hone");
      expect(result.success).toBe(true);
      if (!result.success) return;

      expect(result.file.pragmas).toHaveLength(3);
      expect(result.file.nodes.filter((n) => n.type === "test")).toHaveLength(2);
      expect(result.file.nodes.filter((n) => n.type === "run")).toHaveLength(3);
      expect(result.file.nodes.filter((n) => n.type === "assert")).toHaveLength(7);
    });
  });
});
