/**
 * ANSI Escape Code Utilities
 */

import stripAnsi from "strip-ansi";

/**
 * Remove ANSI escape codes from a string
 */
export function stripAnsiCodes(input: string): string {
  return stripAnsi(input);
}

/**
 * Check if a string contains ANSI escape codes
 */
export function hasAnsiCodes(input: string): boolean {
  return input !== stripAnsi(input);
}
