import stripAnsi from "strip-ansi";

export function stripAnsiCodes(input: string): string {
  return stripAnsi(input);
}

export function hasAnsiCodes(input: string): boolean {
  return input !== stripAnsi(input);
}
