# Hone TextMate Grammar

This document describes the TextMate grammar for Hone `.hone` files.

## Overview

The TextMate grammar (`syntaxes/hone.tmlanguage.json`) provides syntax highlighting for editors that support TextMate grammars (VS Code, Sublime Text, Atom, etc.). It serves as a fallback for editors that don't support LSP semantic tokens.

## Coverage

The grammar provides comprehensive coverage of all Hone syntax elements:

### Keywords and Statements

- **TEST** - Test block declaration
- **RUN** - Command execution (both simple and named forms)
- **ASSERT** - Assertions on command results
- **ENV** - Environment variable declarations

### Pragmas

- **#! shell:** - Shell selection
- **#! env:** - Global environment variables
- **#! timeout:** - Test timeout configuration

### Comments

- **#** - Line comments (excluding pragmas)

### Assertion Targets

- **stdout** - Standard output (ANSI-stripped)
- **stdout.raw** - Raw standard output (with ANSI codes)
- **stderr** - Standard error
- **exit_code** - Command exit code
- **duration** - Command execution duration
- **file** - File system assertions
- **<name>.<target>** - Named command targets (e.g., `build.exit_code`)

### Operators

**Comparison:**
- `==` - Equal
- `!=` - Not equal
- `<` - Less than
- `<=` - Less than or equal
- `>` - Greater than
- `>=` - Greater than or equal

**String:**
- `contains` - String contains substring
- `matches` - String matches regex

**File:**
- `exists` - File exists
- `contains` - File contains string
- `matches` - File matches regex

### Literals

**Strings:**
- Double-quoted: `"hello world"`
- Single-quoted: `'hello world'`
- Escape sequences: `\"`, `\\`, `\n`, `\r`, `\t`, `\xHH`

**Numbers:**
- Integer literals: `0`, `42`, `255`

**Durations:**
- Milliseconds: `100ms`
- Seconds: `5s`
- Minutes: `2m`
- Hours: `1h`

**Regular Expressions:**
- Pattern: `/pattern/`
- With flags: `/pattern/imsx`

## Testing

The grammar has been validated against comprehensive test files including:

1. **Syntax Validation**: All keywords, operators, and literals are recognized
2. **Parser Validation**: Test files parse successfully with the Hone parser
3. **JSON Validation**: Grammar file is valid JSON with correct structure

Test file: `tests/grammar_test.hone`

## Usage

### VS Code

The grammar is automatically loaded when the `.hone` file extension is opened. It's referenced in the VS Code extension configuration.

### Other Editors

Editors that support TextMate grammars can load `syntaxes/hone.tmlanguage.json` directly. Refer to your editor's documentation for TextMate grammar installation.

## Maintenance

When adding new syntax to Hone:

1. Update the grammar in `syntaxes/hone.tmlanguage.json`
2. Add test cases to `tests/grammar_test.hone`
3. Verify the parser handles the new syntax
4. Update this documentation

## Semantic Tokens

For editors that support LSP semantic tokens, the Hone language server provides richer highlighting that supersedes the TextMate grammar. The TextMate grammar serves as a fallback for:

- Editors without LSP support
- Initial file load before LSP is ready
- Syntax highlighting in diffs and previews

## References

- [TextMate Language Grammars](https://macromates.com/manual/en/language_grammars)
- [VS Code Syntax Highlighting Guide](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide)
- [LSP Semantic Tokens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_semanticTokens)
