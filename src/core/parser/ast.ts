/**
 * AST Type Definitions for hone DSL
 *
 * Uses discriminated union types for TypeScript-friendly exhaustive checking.
 * All nodes include line numbers for error diagnostics.
 */

// String literal types
export type StringLiteral = {
  value: string;
  raw: string; // Original string including quotes
  quoteType: "single" | "double";
};

// Regex literal type
export type RegexLiteral = {
  pattern: string;
  flags: string;
  raw: string; // Original regex including slashes
};

// Duration units
export type DurationUnit = "ms" | "s";

// Duration value
export type Duration = {
  value: number;
  unit: DurationUnit;
  raw: string;
};

// Output selectors
export type OutputSelector = "stdout" | "stdout.raw" | "stderr";

// Comparison operators
export type ComparisonOperator = "==" | "!=" | "<" | "<=" | ">" | ">=";

// String comparison operators (subset)
export type StringComparisonOperator = "==" | "!=";

// Assertion predicates for output
export type OutputPredicate =
  | { type: "contains"; value: StringLiteral }
  | { type: "matches"; value: RegexLiteral }
  | { type: "equals"; operator: StringComparisonOperator; value: StringLiteral };

// Assertion predicates for exit code
export type ExitCodePredicate = {
  operator: StringComparisonOperator;
  value: number;
};

// Assertion predicates for duration
export type DurationPredicate = {
  operator: ComparisonOperator;
  value: Duration;
};

// Assertion predicates for files
export type FilePredicate =
  | { type: "exists" }
  | { type: "contains"; value: StringLiteral }
  | { type: "matches"; value: RegexLiteral }
  | { type: "equals"; operator: StringComparisonOperator; value: StringLiteral };

// Assertion types
export type AssertionExpression =
  | {
      type: "output";
      target?: string; // Named RUN reference
      selector: OutputSelector;
      predicate: OutputPredicate;
    }
  | {
      type: "exit_code";
      target?: string; // Named RUN reference
      predicate: ExitCodePredicate;
    }
  | {
      type: "duration";
      target?: string; // Named RUN reference
      predicate: DurationPredicate;
    }
  | {
      type: "file";
      path: StringLiteral;
      predicate: FilePredicate;
    };

// Pragma types
export type PragmaType = "shell" | "env" | "timeout" | "unknown";

export interface PragmaNode {
  type: "pragma";
  pragmaType: PragmaType;
  key?: string; // For env pragma
  value: string;
  line: number;
  raw: string;
}

export interface CommentNode {
  type: "comment";
  text: string;
  line: number;
}

export interface TestNode {
  type: "test";
  name: string;
  line: number;
}

export interface RunNode {
  type: "run";
  name?: string;
  command: string;
  line: number;
}

export interface AssertNode {
  type: "assert";
  expression: AssertionExpression;
  line: number;
  raw: string;
}

export interface EnvNode {
  type: "env";
  key: string;
  value: string;
  line: number;
}

// All possible AST nodes
export type ASTNode =
  | PragmaNode
  | CommentNode
  | TestNode
  | RunNode
  | AssertNode
  | EnvNode;

// Parsed file representation
export interface ParsedFile {
  filename: string;
  pragmas: PragmaNode[];
  nodes: ASTNode[];
  warnings: ParseWarning[];
}

// Parse warning (for unknown pragmas, etc.)
export interface ParseWarning {
  message: string;
  line: number;
  filename: string;
}

// Parse error
export interface ParseError {
  message: string;
  line: number;
  filename: string;
}

// Parse result
export type ParseResult =
  | { success: true; file: ParsedFile }
  | { success: false; errors: ParseError[]; warnings: ParseWarning[] };
