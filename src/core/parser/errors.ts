import type { ParseError, ParseWarning } from "./ast.ts";

export class ParseErrorCollector {
  private errors: ParseError[] = [];
  private warnings: ParseWarning[] = [];
  private filename: string;

  constructor(filename: string) {
    this.filename = filename;
  }

  addError(message: string, line: number): void {
    this.errors.push({
      message,
      line,
      filename: this.filename,
    });
  }

  addWarning(message: string, line: number): void {
    this.warnings.push({
      message,
      line,
      filename: this.filename,
    });
  }

  hasErrors(): boolean {
    return this.errors.length > 0;
  }

  getErrors(): ParseError[] {
    return [...this.errors];
  }

  getWarnings(): ParseWarning[] {
    return [...this.warnings];
  }

  static formatError(error: ParseError): string {
    return `${error.filename}:${error.line} :: ${error.message}`;
  }

  static formatWarning(warning: ParseWarning): string {
    return `${warning.filename}:${warning.line} :: Warning: ${warning.message}`;
  }
}
