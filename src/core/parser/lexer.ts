/**
 * Line-based Lexer for hone DSL
 *
 * Handles tokenization of individual lines and string parsing.
 */

import type {
  StringLiteral,
  RegexLiteral,
  Duration,
  DurationUnit,
} from "./ast.ts";

// Token types
export type TokenType =
  | "PRAGMA"
  | "COMMENT"
  | "TEST"
  | "RUN"
  | "ASSERT"
  | "ENV"
  | "EMPTY"
  | "UNKNOWN";

export interface Token {
  type: TokenType;
  content: string;
  line: number;
}

/**
 * Classify a line into its token type
 */
export function classifyLine(line: string, lineNumber: number): Token {
  const trimmed = line.trim();

  if (trimmed === "") {
    return { type: "EMPTY", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("#!")) {
    return { type: "PRAGMA", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("#")) {
    return { type: "COMMENT", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("TEST ")) {
    return { type: "TEST", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("RUN ")) {
    return { type: "RUN", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("ASSERT ")) {
    return { type: "ASSERT", content: trimmed, line: lineNumber };
  }

  if (trimmed.startsWith("ENV ")) {
    return { type: "ENV", content: trimmed, line: lineNumber };
  }

  return { type: "UNKNOWN", content: trimmed, line: lineNumber };
}

/**
 * Parse a string literal (single or double quoted)
 * Returns the parsed value and the number of characters consumed
 */
export function parseStringLiteral(
  input: string,
  startIndex: number = 0
): { literal: StringLiteral; endIndex: number } | null {
  const startChar = input[startIndex];

  if (startChar !== '"' && startChar !== "'") {
    return null;
  }

  const quoteType = startChar === '"' ? "double" : "single";
  let value = "";
  let i = startIndex + 1;
  let escaped = false;

  while (i < input.length) {
    const char = input[i];

    if (escaped) {
      if (quoteType === "double") {
        // Handle escape sequences in double-quoted strings
        switch (char) {
          case "n":
            value += "\n";
            break;
          case "t":
            value += "\t";
            break;
          case '"':
            value += '"';
            break;
          case "\\":
            value += "\\";
            break;
          default:
            // Unknown escape, keep as-is
            value += "\\" + char;
        }
      } else {
        // Single quotes: no escape sequences, literal backslash
        value += "\\" + char;
      }
      escaped = false;
    } else if (char === "\\") {
      escaped = true;
    } else if (char === startChar) {
      // End of string
      return {
        literal: {
          value,
          raw: input.substring(startIndex, i + 1),
          quoteType,
        },
        endIndex: i + 1,
      };
    } else {
      value += char;
    }
    i++;
  }

  // Unterminated string
  return null;
}

/**
 * Parse a regex literal (/pattern/flags)
 */
export function parseRegexLiteral(
  input: string,
  startIndex: number = 0
): { literal: RegexLiteral; endIndex: number } | null {
  if (input[startIndex] !== "/") {
    return null;
  }

  let pattern = "";
  let i = startIndex + 1;
  let escaped = false;

  // Parse pattern
  while (i < input.length) {
    const char = input[i];

    if (escaped) {
      pattern += char;
      escaped = false;
    } else if (char === "\\") {
      pattern += char;
      escaped = true;
    } else if (char === "/") {
      // End of pattern, parse flags
      i++;
      let flags = "";
      while (i < input.length && /[gimsuy]/.test(input[i]!)) {
        flags += input[i];
        i++;
      }

      return {
        literal: {
          pattern,
          flags,
          raw: input.substring(startIndex, i),
        },
        endIndex: i,
      };
    } else {
      pattern += char;
    }
    i++;
  }

  // Unterminated regex
  return null;
}

/**
 * Parse a duration value (e.g., "200ms", "1.5s")
 */
export function parseDuration(
  input: string,
  startIndex: number = 0
): { duration: Duration; endIndex: number } | null {
  let i = startIndex;

  // Skip whitespace
  while (i < input.length && input[i] === " ") {
    i++;
  }

  const numStart = i;

  // Parse number (including decimal)
  while (
    i < input.length &&
    (/\d/.test(input[i]!) || input[i] === ".")
  ) {
    i++;
  }

  if (i === numStart) {
    return null;
  }

  const numStr = input.substring(numStart, i);
  const value = parseFloat(numStr);

  if (isNaN(value)) {
    return null;
  }

  // Parse unit
  const unitStart = i;
  while (i < input.length && /[a-z]/.test(input[i]!)) {
    i++;
  }

  const unit = input.substring(unitStart, i) as DurationUnit;

  if (unit !== "ms" && unit !== "s") {
    return null;
  }

  return {
    duration: {
      value,
      unit,
      raw: input.substring(startIndex, i).trim(),
    },
    endIndex: i,
  };
}

/**
 * Parse a number
 */
export function parseNumber(
  input: string,
  startIndex: number = 0
): { value: number; endIndex: number } | null {
  let i = startIndex;

  // Skip whitespace
  while (i < input.length && input[i] === " ") {
    i++;
  }

  const numStart = i;

  // Handle negative numbers
  if (input[i] === "-") {
    i++;
  }

  // Parse digits
  while (i < input.length && /\d/.test(input[i]!)) {
    i++;
  }

  if (i === numStart || (i === numStart + 1 && input[numStart] === "-")) {
    return null;
  }

  const value = parseInt(input.substring(numStart, i), 10);

  if (isNaN(value)) {
    return null;
  }

  return { value, endIndex: i };
}

/**
 * Skip whitespace in input
 */
export function skipWhitespace(input: string, startIndex: number): number {
  let i = startIndex;
  while (i < input.length && input[i] === " ") {
    i++;
  }
  return i;
}

/**
 * Match a word at current position
 */
export function matchWord(
  input: string,
  startIndex: number,
  word: string
): boolean {
  const slice = input.substring(startIndex, startIndex + word.length);
  if (slice !== word) {
    return false;
  }
  // Ensure word boundary
  const nextChar = input[startIndex + word.length];
  return nextChar === undefined || nextChar === " " || nextChar === ".";
}

/**
 * Parse a comparison operator
 */
export function parseComparisonOperator(
  input: string,
  startIndex: number
): { operator: string; endIndex: number } | null {
  const operators = ["==", "!=", "<=", ">=", "<", ">"];
  let i = skipWhitespace(input, startIndex);

  for (const op of operators) {
    if (input.substring(i, i + op.length) === op) {
      return { operator: op, endIndex: i + op.length };
    }
  }

  return null;
}
