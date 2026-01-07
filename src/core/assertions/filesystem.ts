import { readFile, stat, realpath } from "node:fs/promises";
import { resolve, basename } from "node:path";
import type { FilePredicate, StringLiteral, RegexLiteral } from "../parser/index.ts";
import type { AssertionResult } from "./output.ts";

export async function evaluateFilePredicate(
  filePath: StringLiteral,
  predicate: FilePredicate,
  cwd: string
): Promise<AssertionResult> {
  // Resolve path against cwd
  const resolvedPath = resolve(cwd, filePath.value);

  switch (predicate.type) {
    case "exists":
      return evaluateFileExists(resolvedPath, filePath.raw, cwd);
    case "contains":
      return evaluateFileContains(resolvedPath, predicate.value, filePath.raw, cwd);
    case "matches":
      return evaluateFileMatches(resolvedPath, predicate.value, filePath.raw, cwd);
    case "equals":
      return evaluateFileEquals(
        resolvedPath,
        predicate.operator,
        predicate.value,
        filePath.raw,
        cwd
      );
  }
}

async function checkFileExists(
  filePath: string,
  _cwd: string
): Promise<{ exists: boolean; casingMatch: boolean; actualName?: string }> {
  try {
    await stat(filePath);

    // Check for case mismatch on case-insensitive filesystems
    const realPath = await realpath(filePath);
    const expectedName = basename(filePath);
    const actualName = basename(realPath);

    if (expectedName !== actualName) {
      return { exists: true, casingMatch: false, actualName };
    }

    return { exists: true, casingMatch: true };
  } catch {
    return { exists: false, casingMatch: true };
  }
}

async function evaluateFileExists(
  filePath: string,
  pathRaw: string,
  cwd: string
): Promise<AssertionResult> {
  const check = await checkFileExists(filePath, cwd);

  if (check.exists && !check.casingMatch) {
    return {
      passed: false,
      expected: `file ${pathRaw} to exist`,
      actual: `file exists but with different casing: "${check.actualName}"`,
      error: `Case mismatch: expected "${basename(filePath)}" but found "${check.actualName}"`,
    };
  }

  return {
    passed: check.exists,
    expected: `file ${pathRaw} to exist`,
    actual: check.exists ? "file exists" : "file does not exist",
  };
}

async function readFileContent(
  filePath: string,
  pathRaw: string,
  cwd: string
): Promise<{ content: string; error?: AssertionResult }> {
  const check = await checkFileExists(filePath, cwd);

  if (!check.exists) {
    return {
      content: "",
      error: {
        passed: false,
        expected: `file ${pathRaw} to exist`,
        actual: "file does not exist",
      },
    };
  }

  if (!check.casingMatch) {
    return {
      content: "",
      error: {
        passed: false,
        expected: `file ${pathRaw} to exist with exact casing`,
        actual: `file exists but with different casing: "${check.actualName}"`,
        error: `Case mismatch: expected "${basename(filePath)}" but found "${check.actualName}"`,
      },
    };
  }

  try {
    const content = await readFile(filePath, "utf-8");
    return { content };
  } catch (e) {
    return {
      content: "",
      error: {
        passed: false,
        expected: `to read file ${pathRaw}`,
        actual: `failed to read file: ${(e as Error).message}`,
      },
    };
  }
}

async function evaluateFileContains(
  filePath: string,
  value: StringLiteral,
  pathRaw: string,
  cwd: string
): Promise<AssertionResult> {
  const { content, error } = await readFileContent(filePath, pathRaw, cwd);
  if (error) return error;

  const passed = content.includes(value.value);
  return {
    passed,
    expected: `file ${pathRaw} to contain ${value.raw}`,
    actual: content,
  };
}

async function evaluateFileMatches(
  filePath: string,
  value: RegexLiteral,
  pathRaw: string,
  cwd: string
): Promise<AssertionResult> {
  const { content, error } = await readFileContent(filePath, pathRaw, cwd);
  if (error) return error;

  try {
    const regex = new RegExp(value.pattern, value.flags);
    const passed = regex.test(content);
    return {
      passed,
      expected: `file ${pathRaw} to match ${value.raw}`,
      actual: content,
    };
  } catch (e) {
    return {
      passed: false,
      expected: `file ${pathRaw} to match ${value.raw}`,
      actual: content,
      error: `Invalid regex: ${(e as Error).message}`,
    };
  }
}

function normalizeFileContent(content: string): string {
  return content
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();
}

async function evaluateFileEquals(
  filePath: string,
  operator: "==" | "!=",
  value: StringLiteral,
  pathRaw: string,
  cwd: string
): Promise<AssertionResult> {
  const { content, error } = await readFileContent(filePath, pathRaw, cwd);
  if (error) return error;

  const normalizedContent = normalizeFileContent(content);
  const normalizedValue = normalizeFileContent(value.value);

  const isEqual = normalizedContent === normalizedValue;
  const passed = operator === "==" ? isEqual : !isEqual;

  return {
    passed,
    expected: `file ${pathRaw} ${operator} ${value.raw}`,
    actual: content,
  };
}
