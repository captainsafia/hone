import { describe, test, expect } from "bun:test";
import { createShellConfig, isShellSupported } from "../../src/core/runner/shell.ts";
import type { PragmaNode } from "../../src/core/parser/ast.ts";

describe("Shell Configuration", () => {
  describe("createShellConfig", () => {
    test("uses default shell when no pragma", () => {
      const config = createShellConfig([], "test.hone", "/home/user");
      expect(config.shell).toBeDefined();
      expect(config.timeout).toBe(30000);
      expect(config.cwd).toBe("/home/user");
      expect(config.filename).toBe("test.hone");
    });

    test("uses shell from pragma", () => {
      const pragmas: PragmaNode[] = [
        {
          type: "pragma",
          pragmaType: "shell",
          value: "/bin/zsh",
          line: 1,
          raw: "#! shell: /bin/zsh",
        },
      ];
      const config = createShellConfig(pragmas, "test.hone", "/home/user");
      expect(config.shell).toBe("/bin/zsh");
    });

    test("uses override shell over pragma", () => {
      const pragmas: PragmaNode[] = [
        {
          type: "pragma",
          pragmaType: "shell",
          value: "/bin/zsh",
          line: 1,
          raw: "#! shell: /bin/zsh",
        },
      ];
      const config = createShellConfig(
        pragmas,
        "test.hone",
        "/home/user",
        "/bin/bash"
      );
      expect(config.shell).toBe("/bin/bash");
    });

    test("parses timeout in seconds", () => {
      const pragmas: PragmaNode[] = [
        {
          type: "pragma",
          pragmaType: "timeout",
          value: "60s",
          line: 1,
          raw: "#! timeout: 60s",
        },
      ];
      const config = createShellConfig(pragmas, "test.hone", "/home/user");
      expect(config.timeout).toBe(60000);
    });

    test("parses timeout in milliseconds", () => {
      const pragmas: PragmaNode[] = [
        {
          type: "pragma",
          pragmaType: "timeout",
          value: "500ms",
          line: 1,
          raw: "#! timeout: 500ms",
        },
      ];
      const config = createShellConfig(pragmas, "test.hone", "/home/user");
      expect(config.timeout).toBe(500);
    });

    test("collects env pragmas", () => {
      const pragmas: PragmaNode[] = [
        {
          type: "pragma",
          pragmaType: "env",
          key: "FOO",
          value: "bar",
          line: 1,
          raw: "#! env: FOO=bar",
        },
        {
          type: "pragma",
          pragmaType: "env",
          key: "DEBUG",
          value: "true",
          line: 2,
          raw: "#! env: DEBUG=true",
        },
      ];
      const config = createShellConfig(pragmas, "test.hone", "/home/user");
      expect(config.env["FOO"]).toBe("bar");
      expect(config.env["DEBUG"]).toBe("true");
    });
  });

  describe("isShellSupported", () => {
    test("supports bash", () => {
      expect(isShellSupported("/bin/bash")).toBe(true);
    });

    test("supports zsh", () => {
      expect(isShellSupported("/usr/bin/zsh")).toBe(true);
    });

    test("supports sh", () => {
      expect(isShellSupported("/bin/sh")).toBe(true);
    });

    test("returns false for unknown shell", () => {
      expect(isShellSupported("/bin/custom-shell")).toBe(false);
    });
  });
});
