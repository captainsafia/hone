import type {
  ASTNode,
  PragmaNode,
  TestNode,
  RunNode,
  AssertNode,
  EnvNode,
  CommentNode,
  ParseResult,
  AssertionExpression,
  OutputSelector,
  StringComparisonOperator,
  ComparisonOperator,
} from "./ast.ts";
import { ParseErrorCollector } from "./errors.ts";
import {
  classifyLine,
  parseStringLiteral,
  parseRegexLiteral,
  parseDuration,
  parseNumber,
  skipWhitespace,
  matchWord,
  parseComparisonOperator,
} from "./lexer.ts";

export * from "./ast.ts";
export * from "./errors.ts";

export function parseFile(content: string, filename: string): ParseResult {
  const lines = content.split("\n");
  const collector = new ParseErrorCollector(filename);
  const pragmas: PragmaNode[] = [];
  const nodes: ASTNode[] = [];
  const runNames = new Set<string>();

  let inPragmaSection = true;

  for (let i = 0; i < lines.length; i++) {
    const lineNumber = i + 1;
    const line = lines[i]!;
    const token = classifyLine(line, lineNumber);

    switch (token.type) {
      case "EMPTY":
        // Skip empty lines
        break;

      case "COMMENT":
        nodes.push({
          type: "comment",
          text: token.content.substring(1).trim(),
          line: lineNumber,
        } satisfies CommentNode);
        break;

      case "PRAGMA": {
        if (!inPragmaSection) {
          collector.addError(
            "Pragmas must appear at the top of the file",
            lineNumber
          );
          break;
        }

        const pragma = parsePragma(token.content, lineNumber, collector);
        if (pragma) {
          pragmas.push(pragma);
          nodes.push(pragma);
        }
        break;
      }

      case "TEST": {
        inPragmaSection = false;
        const test = parseTest(token.content, lineNumber, collector);
        if (test) {
          nodes.push(test);
        }
        break;
      }

      case "RUN": {
        inPragmaSection = false;
        const run = parseRun(token.content, lineNumber, collector, runNames);
        if (run) {
          nodes.push(run);
        }
        break;
      }

      case "ASSERT": {
        inPragmaSection = false;
        const assert = parseAssert(token.content, lineNumber, collector);
        if (assert) {
          nodes.push(assert);
        }
        break;
      }

      case "ENV": {
        inPragmaSection = false;
        const env = parseEnv(token.content, lineNumber, collector);
        if (env) {
          nodes.push(env);
        }
        break;
      }

      case "UNKNOWN":
        inPragmaSection = false;
        collector.addError(`Unknown statement: ${token.content}`, lineNumber);
        break;
    }
  }

  if (collector.hasErrors()) {
    return {
      success: false,
      errors: collector.getErrors(),
      warnings: collector.getWarnings(),
    };
  }

  return {
    success: true,
    file: {
      filename,
      pragmas,
      nodes,
      warnings: collector.getWarnings(),
    },
  };
}

function parsePragma(
  content: string,
  line: number,
  collector: ParseErrorCollector
): PragmaNode | null {
  // Remove #! prefix
  const rest = content.substring(2).trim();

  // Parse key: value
  const colonIndex = rest.indexOf(":");
  if (colonIndex === -1) {
    collector.addError(`Invalid pragma syntax: ${content}`, line);
    return null;
  }

  const pragmaKey = rest.substring(0, colonIndex).trim().toLowerCase();
  const pragmaValue = rest.substring(colonIndex + 1).trim();

  switch (pragmaKey) {
    case "shell":
      return {
        type: "pragma",
        pragmaType: "shell",
        value: pragmaValue,
        line,
        raw: content,
      };

    case "env": {
      // Parse KEY=value
      const eqIndex = pragmaValue.indexOf("=");
      if (eqIndex === -1) {
        collector.addError(`Invalid env pragma: ${content}`, line);
        return null;
      }
      const envKey = pragmaValue.substring(0, eqIndex).trim();
      const envValue = pragmaValue.substring(eqIndex + 1);
      return {
        type: "pragma",
        pragmaType: "env",
        key: envKey,
        value: envValue,
        line,
        raw: content,
      };
    }

    case "timeout": {
      // Validate timeout format
      const result = parseDuration(pragmaValue, 0);
      if (!result) {
        collector.addError(
          `Invalid timeout format: ${pragmaValue}. Expected format: <number>s or <number>ms`,
          line
        );
        return null;
      }
      return {
        type: "pragma",
        pragmaType: "timeout",
        value: pragmaValue,
        line,
        raw: content,
      };
    }

    default:
      // Unknown pragma - warn but continue
      collector.addWarning(`Unknown pragma: ${pragmaKey}`, line);
      return {
        type: "pragma",
        pragmaType: "unknown",
        value: rest,
        line,
        raw: content,
      };
  }
}

function parseTest(
  content: string,
  line: number,
  collector: ParseErrorCollector
): TestNode | null {
  // TEST "name"
  const rest = content.substring(5); // After "TEST "
  const result = parseStringLiteral(rest, 0);

  if (!result) {
    collector.addError(
      `Invalid TEST syntax: expected quoted test name`,
      line
    );
    return null;
  }

  const name = result.literal.value;

  // Validate name characters
  if (!/^[a-zA-Z0-9 _-]+$/.test(name)) {
    collector.addError(
      `Invalid test name: "${name}". Names can only contain alphanumeric characters, spaces, dashes, and underscores`,
      line
    );
    return null;
  }

  return {
    type: "test",
    name,
    line,
  };
}

function parseRun(
  content: string,
  line: number,
  collector: ParseErrorCollector,
  runNames: Set<string>
): RunNode | null {
  // RUN <command> or RUN <name>: <command>
  const rest = content.substring(4); // After "RUN "

  // Check for named RUN (name: command)
  const colonMatch = rest.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):\s*/);

  if (colonMatch) {
    const name = colonMatch[1]!;
    const command = rest.substring(colonMatch[0].length);

    if (runNames.has(name)) {
      collector.addError(
        `Duplicate RUN name: "${name}". RUN names must be unique across the entire file`,
        line
      );
      return null;
    }

    runNames.add(name);

    if (!command.trim()) {
      collector.addError(`Empty command in RUN statement`, line);
      return null;
    }

    return {
      type: "run",
      name,
      command: command.trim(),
      line,
    };
  }

  // Unnamed RUN
  if (!rest.trim()) {
    collector.addError(`Empty command in RUN statement`, line);
    return null;
  }

  return {
    type: "run",
    command: rest.trim(),
    line,
  };
}

function parseEnv(
  content: string,
  line: number,
  collector: ParseErrorCollector
): EnvNode | null {
  // ENV KEY=value
  const rest = content.substring(4); // After "ENV "
  const eqIndex = rest.indexOf("=");

  if (eqIndex === -1) {
    collector.addError(`Invalid ENV syntax: expected KEY=value`, line);
    return null;
  }

  const key = rest.substring(0, eqIndex).trim();
  const value = rest.substring(eqIndex + 1);

  if (!key) {
    collector.addError(`Invalid ENV syntax: empty key`, line);
    return null;
  }

  // Validate key format (valid environment variable name)
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(key)) {
    collector.addError(
      `Invalid environment variable name: "${key}". Names must start with a letter or underscore and contain only alphanumeric characters and underscores`,
      line
    );
    return null;
  }

  return {
    type: "env",
    key,
    value,
    line,
  };
}

function parseAssert(
  content: string,
  line: number,
  collector: ParseErrorCollector
): AssertNode | null {
  // ASSERT <expression>
  const rest = content.substring(7); // After "ASSERT "
  const expression = parseAssertionExpression(rest, line, collector);

  if (!expression) {
    return null;
  }

  return {
    type: "assert",
    expression,
    line,
    raw: content,
  };
}

function parseAssertionExpression(
  input: string,
  line: number,
  collector: ParseErrorCollector
): AssertionExpression | null {
  let i = 0;
  i = skipWhitespace(input, i);

  // Check for file assertion
  if (matchWord(input, i, "file")) {
    return parseFileAssertion(input, line, collector);
  }

  // Check for stdout.raw first (before named target check, since stdout.raw
  // would otherwise match the named target pattern)
  if (matchWord(input, i, "stdout.raw")) {
    i += 10; // "stdout.raw"
    return parseOutputAssertion(input, i, "stdout.raw", undefined, line, collector);
  }

  // Check for named target (e.g., build.stdout)
  let target: string | undefined;
  const dotMatch = input.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\.(.+)/);

  if (dotMatch) {
    // But we need to check that this isn't stdout, stderr, exit_code, or duration
    const potentialTarget = dotMatch[1];
    if (
      potentialTarget !== "stdout" &&
      potentialTarget !== "stderr" &&
      potentialTarget !== "exit_code" &&
      potentialTarget !== "duration"
    ) {
      target = potentialTarget;
      input = dotMatch[2]!;
      i = 0;
    }
  }

  // Parse selector
  if (matchWord(input, i, "stdout.raw")) {
    i += 10; // "stdout.raw"
    return parseOutputAssertion(input, i, "stdout.raw", target, line, collector);
  }

  if (matchWord(input, i, "stdout")) {
    i += 6;
    return parseOutputAssertion(input, i, "stdout", target, line, collector);
  }

  if (matchWord(input, i, "stderr")) {
    i += 6;
    return parseOutputAssertion(input, i, "stderr", target, line, collector);
  }

  if (matchWord(input, i, "exit_code")) {
    i += 9;
    return parseExitCodeAssertion(input, i, target, line, collector);
  }

  if (matchWord(input, i, "duration")) {
    i += 8;
    return parseDurationAssertion(input, i, target, line, collector);
  }

  collector.addError(`Unknown assertion type: ${input}`, line);
  return null;
}

function parseOutputAssertion(
  input: string,
  startIndex: number,
  selector: OutputSelector,
  target: string | undefined,
  line: number,
  collector: ParseErrorCollector
): AssertionExpression | null {
  let i = skipWhitespace(input, startIndex);

  // Parse predicate
  if (matchWord(input, i, "contains")) {
    i += 8;
    i = skipWhitespace(input, i);

    const strResult = parseStringLiteral(input, i);
    if (!strResult) {
      collector.addError(
        `Expected quoted string after "contains"`,
        line
      );
      return null;
    }

    return {
      type: "output",
      target,
      selector,
      predicate: { type: "contains", value: strResult.literal },
    };
  }

  if (matchWord(input, i, "matches")) {
    i += 7;
    i = skipWhitespace(input, i);

    const regexResult = parseRegexLiteral(input, i);
    if (!regexResult) {
      collector.addError(
        `Expected regex literal after "matches"`,
        line
      );
      return null;
    }

    return {
      type: "output",
      target,
      selector,
      predicate: { type: "matches", value: regexResult.literal },
    };
  }

  // Check for == or !=
  const opResult = parseComparisonOperator(input, i);
  if (opResult && (opResult.operator === "==" || opResult.operator === "!=")) {
    i = skipWhitespace(input, opResult.endIndex);

    const strResult = parseStringLiteral(input, i);
    if (!strResult) {
      collector.addError(
        `Expected quoted string after "${opResult.operator}"`,
        line
      );
      return null;
    }

    return {
      type: "output",
      target,
      selector,
      predicate: {
        type: "equals",
        operator: opResult.operator as StringComparisonOperator,
        value: strResult.literal,
      },
    };
  }

  collector.addError(
    `Expected predicate (contains, matches, ==, !=) after "${selector}"`,
    line
  );
  return null;
}

function parseExitCodeAssertion(
  input: string,
  startIndex: number,
  target: string | undefined,
  line: number,
  collector: ParseErrorCollector
): AssertionExpression | null {
  const opResult = parseComparisonOperator(input, startIndex);

  if (!opResult || (opResult.operator !== "==" && opResult.operator !== "!=")) {
    collector.addError(
      `Expected == or != after "exit_code"`,
      line
    );
    return null;
  }

  const numResult = parseNumber(input, opResult.endIndex);
  if (!numResult) {
    collector.addError(
      `Expected number after "${opResult.operator}"`,
      line
    );
    return null;
  }

  return {
    type: "exit_code",
    target,
    predicate: {
      operator: opResult.operator as StringComparisonOperator,
      value: numResult.value,
    },
  };
}

function parseDurationAssertion(
  input: string,
  startIndex: number,
  target: string | undefined,
  line: number,
  collector: ParseErrorCollector
): AssertionExpression | null {
  const opResult = parseComparisonOperator(input, startIndex);

  if (!opResult) {
    collector.addError(`Expected comparison operator after "duration"`, line);
    return null;
  }

  const durationResult = parseDuration(input, opResult.endIndex);
  if (!durationResult) {
    collector.addError(
      `Expected duration value (e.g., 200ms, 1.5s) after "${opResult.operator}"`,
      line
    );
    return null;
  }

  return {
    type: "duration",
    target,
    predicate: {
      operator: opResult.operator as ComparisonOperator,
      value: durationResult.duration,
    },
  };
}

function parseFileAssertion(
  input: string,
  line: number,
  collector: ParseErrorCollector
): AssertionExpression | null {
  // file "path" <predicate>
  let i = 4; // After "file"
  i = skipWhitespace(input, i);

  const pathResult = parseStringLiteral(input, i);
  if (!pathResult) {
    collector.addError(`Expected quoted file path after "file"`, line);
    return null;
  }

  i = skipWhitespace(input, pathResult.endIndex);

  // Parse predicate
  if (matchWord(input, i, "exists")) {
    return {
      type: "file",
      path: pathResult.literal,
      predicate: { type: "exists" },
    };
  }

  if (matchWord(input, i, "contains")) {
    i += 8;
    i = skipWhitespace(input, i);

    const strResult = parseStringLiteral(input, i);
    if (!strResult) {
      collector.addError(`Expected quoted string after "contains"`, line);
      return null;
    }

    return {
      type: "file",
      path: pathResult.literal,
      predicate: { type: "contains", value: strResult.literal },
    };
  }

  if (matchWord(input, i, "matches")) {
    i += 7;
    i = skipWhitespace(input, i);

    const regexResult = parseRegexLiteral(input, i);
    if (!regexResult) {
      collector.addError(`Expected regex literal after "matches"`, line);
      return null;
    }

    return {
      type: "file",
      path: pathResult.literal,
      predicate: { type: "matches", value: regexResult.literal },
    };
  }

  // Check for == or !=
  const opResult = parseComparisonOperator(input, i);
  if (opResult && (opResult.operator === "==" || opResult.operator === "!=")) {
    i = skipWhitespace(input, opResult.endIndex);

    const strResult = parseStringLiteral(input, i);
    if (!strResult) {
      collector.addError(
        `Expected quoted string after "${opResult.operator}"`,
        line
      );
      return null;
    }

    return {
      type: "file",
      path: pathResult.literal,
      predicate: {
        type: "equals",
        operator: opResult.operator as StringComparisonOperator,
        value: strResult.literal,
      },
    };
  }

  collector.addError(
    `Expected predicate (exists, contains, matches, ==, !=) after file path`,
    line
  );
  return null;
}
