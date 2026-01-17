# Hone LSP Implementation Specification

## Overview

This document specifies the Language Server Protocol (LSP) implementation for Hone, a Rust-based CLI tool for integration testing of command-line applications. The LSP will provide IDE features for `.hone` test files.

## Architecture

### Binary Model

The LSP server is integrated into the main `hone` binary as a subcommand:

```sh
hone lsp
```

This shares code with the main CLI and simplifies distribution.

### Code Location

LSP-specific code resides in `src/lsp/` as a top-level module alongside `parser/`, `runner/`, etc.

### LSP Library

Uses the `async-lsp` crate for the LSP protocol implementation. This integrates well with the existing Tokio async runtime.

### Target Editors

The implementation targets multiple editors (VS Code, Neovim, Helix, Zed, etc.) and prioritizes strict LSP specification compliance over editor-specific features.

## Parser Requirements

### Fault-Tolerant Incremental Parser

**This is a blocking prerequisite for the LSP implementation.**

The existing parser in `src/parser/` must be upgraded to support:

1. **Fault Tolerance**: Continue parsing after errors, recovering at block boundaries (`@test`, `@setup`)
2. **Error Nodes**: Insert explicit `Error` AST nodes with source spans for unparseable sections
3. **Incremental Updates**: File-level granularity (reparse entire file on any change)

The upgraded parser will be shared between the CLI test runner and the LSP server.

### Error Recovery Strategy

- On parse error, skip to the next `@test` or `@setup` boundary
- Insert an `Error` node in the AST with the span of the unparseable content
- Continue parsing subsequent blocks
- Report multiple diagnostics per file (one per error node)

## LSP Capabilities

### Diagnostics

**Level**: Syntax + Semantic + Type Checking

Diagnostics are provided for:

1. **Syntax Errors**: Invalid `.hone` file syntax
2. **Semantic Errors**: Invalid assertion names, malformed block structure
3. **Type Errors**: Assertion arguments don't match expected types

Diagnostics include fix suggestions in the message text (not code actions).

**Version Compatibility**: Unknown syntax from newer Hone versions triggers graceful degradation—skip the unknown block and continue analyzing the rest of the file.

### Completion

**Style**: Context-aware snippets with tab stops

Completions are provided for:

- Keywords (`@test`, `@setup`, `expect`, etc.)
- Assertion names (`stdout`, `stderr`, `exitcode`, etc.)
- Shell commands (hardcoded common commands + PATH scanning)

**Trigger Behavior**: After typing `expect ` (with trailing space), immediately show available assertions.

**Snippet Examples**:

```
expect stdout {
  $1
}

expect exitcode $1

@test "$1" {
  $2
}
```

Snippets adapt based on surrounding context.

### Shell Awareness

**Level**: Basic awareness

- Knows common shells (bash, zsh, fish)
- Provides completion hints for common shell commands
- Shell command source: hardcoded common commands (ls, cd, git, grep, etc.) plus executables discovered via PATH scanning
- Shell commands within tests are otherwise treated as opaque strings

### Hover Information

**Level**: Keyword documentation

Hovering over keywords (`@test`, `@setup`, `expect`) and assertion names shows documentation.

**Documentation Source**: Generated from the parser's AST type definitions at build time.

### Document Symbols (Outline)

**Level**: Full outline

Provides document symbols for:

- `@test` blocks (with test name)
- `@setup` blocks
- Nested structure within blocks

### Formatting

**Level**: Basic formatting

Provides:

- Consistent indentation
- Normalized spacing

**Shell Command Indentation**: Minimal indent (single indent level for shell commands within test blocks). Original shell command internal formatting is preserved.

### Syntax Highlighting

**Strategy**: Both TextMate and semantic tokens

- TextMate grammars serve as fallback for editors without semantic token support
- LSP provides semantic tokens for richer highlighting when supported

### Navigation

**Not Implemented**: No "Go to Definition" or cross-file navigation in this version.

### Test Result Integration

**Not Implemented**: The LSP is purely static analysis; it does not track or display test pass/fail status.

## Configuration

**Level**: Zero configuration

The LSP uses sensible defaults with no configuration options. No initialization options, no config files.

## Workspace Model

**Level**: Lazy workspace

- Discovers `.hone` files in the workspace but only parses them when opened
- Operates primarily on currently open files
- No eager indexing on startup

## Logging

**Destination**: File logging

Logs are written to a file in a standard location for debugging purposes. The specific location follows platform conventions (e.g., `~/.local/state/hone/lsp.log` on Linux).

## Testing Strategy

### Unit Tests

- Test parser fault tolerance in isolation
- Test semantic analyzer logic
- Test completion provider logic

### Mock LSP Client

- Test against simulated LSP protocol messages
- Verify correct responses to requests
- Test error handling and edge cases

No full integration tests spawning real LSP servers required.

## Implementation Phases

### Phase 1: Parser Upgrades (Blocking)

1. Add error recovery to lexer
2. Implement block-boundary recovery in parser
3. Add `Error` AST node type with source spans
4. Ensure existing tests pass
5. Add tests for fault tolerance

### Phase 2: Core LSP Infrastructure

1. Add `async-lsp` dependency
2. Implement `hone lsp` subcommand
3. Set up file logging
4. Implement basic lifecycle (initialize, shutdown)
5. Implement text document synchronization

### Phase 3: Diagnostics

1. Wire parser errors to LSP diagnostics
2. Implement semantic analysis
3. Implement type checking for assertions
4. Add fix suggestions to diagnostic messages

### Phase 4: Completion

1. Implement keyword completion
2. Implement assertion name completion
3. Add snippet support with tab stops
4. Implement context-aware snippet adaptation
5. Add shell command completion (common + PATH)

### Phase 5: Additional Features

1. Implement hover with keyword documentation
2. Implement document symbols (outline)
3. Implement basic formatting
4. Implement semantic tokens

### Phase 6: Polish

1. Documentation
2. TextMate grammar for `.hone` files
3. Example editor configurations

## Module Structure

```
src/lsp/
├── mod.rs              # Module exports, server entry point
├── server.rs           # LSP server implementation
├── handlers.rs         # Request/notification handlers
├── diagnostics.rs      # Diagnostic generation
├── completion.rs       # Completion provider
├── hover.rs            # Hover provider
├── symbols.rs          # Document symbols provider
├── formatting.rs       # Formatting provider
├── semantic_tokens.rs  # Semantic token provider
└── shell.rs            # Shell command knowledge
```

## Dependencies

New dependencies to add:

- `async-lsp` - LSP protocol implementation
- `tracing` / `tracing-appender` - File logging (if not already present)

## Success Criteria

1. LSP starts via `hone lsp` and communicates over stdio
2. Syntax errors in `.hone` files appear as diagnostics
3. Semantic and type errors are reported
4. Completion works for keywords, assertions, and shell commands
5. Snippets expand with tab stops
6. Hover shows documentation for keywords
7. Outline view shows test structure
8. Basic formatting normalizes indentation
9. Works correctly in VS Code, Neovim, and Helix
10. All unit and mock client tests pass
11. Existing `cargo test` suite passes without regressions
12. The `hone` CLI continues to function correctly (all integration tests pass)

## Implementation Plan

This plan provides detailed, actionable tasks organized by phase. Each task maps to one or more success criteria (referenced as **SC#**).

---

### Phase 1: Parser Upgrades (Blocking)

**Goal**: Make the parser fault-tolerant so it can provide useful information even for incomplete or invalid files.

#### 1.1 Lexer Error Recovery
- [x] Modify `lexer.rs` to emit an `Error` token type instead of panicking on unexpected characters
- [x] Continue lexing after encountering an error token
- [x] Track error spans for precise diagnostic reporting
- [x] Add unit tests for lexer error recovery (invalid characters, unterminated strings)

#### 1.2 Parser Error Recovery
- [x] Add `Error` variant to the AST node types in `ast.rs` with associated `Span`
- [x] Implement synchronization points at block boundaries (`@test`, `@setup`, EOF)
- [x] On parse error, consume tokens until the next sync point and emit an `Error` node
- [x] Collect all errors into a vector instead of returning on first error
- [x] Add unit tests for parser recovery (incomplete blocks, missing braces, invalid keywords)

#### 1.3 Span Tracking Enhancement
- [x] Ensure all AST nodes carry accurate `Span` information (start/end positions)
- [x] Include byte offsets, line numbers, and column numbers in spans
- [x] Verify spans are correct for both valid and error nodes

#### 1.4 Validation
- [x] Ensure all existing CLI tests pass with the updated parser
- [x] Add integration tests using malformed `.hone` files that verify partial parsing works

**Criteria addressed**: SC#2 (syntax errors as diagnostics)

---

### Phase 2: Core LSP Infrastructure

**Goal**: Establish the LSP server skeleton that can start, communicate, and shut down correctly.

#### 2.1 Add Dependencies
- [x] Add `async-lsp` to `Cargo.toml`
- [x] Add `tracing` and `tracing-appender` for file logging
- [x] Verify dependencies compile and don't conflict with existing crates

#### 2.2 Create LSP Module Structure
- [x] Create `src/lsp/mod.rs` with module declarations
- [x] Create stub files: `server.rs`, `handlers.rs`
- [x] Export the LSP entry point from `lib.rs`

#### 2.3 Implement `hone lsp` Subcommand
- [x] Add `lsp` subcommand to CLI argument parser in `main.rs`
- [x] Wire subcommand to LSP server entry point
- [x] Ensure `hone lsp` starts without errors

#### 2.4 Set Up File Logging
- [x] Configure `tracing` to write to `~/.local/state/hone/lsp.log` (Linux) or platform equivalent
- [x] Implement log rotation or size limits to prevent unbounded growth
- [x] Add startup log message with version info

#### 2.5 Implement LSP Lifecycle
- [x] Handle `initialize` request: return server capabilities
- [x] Handle `initialized` notification
- [x] Handle `shutdown` request and `exit` notification
- [x] Test lifecycle with a mock client

#### 2.6 Implement Text Document Synchronization
- [x] Handle `textDocument/didOpen`: store document content
- [x] Handle `textDocument/didChange`: update stored content (full sync mode)
- [x] Handle `textDocument/didClose`: remove document from memory
- [x] Maintain a document store (`HashMap<Uri, DocumentState>`)

**Criteria addressed**: SC#1 (LSP starts and communicates over stdio)

---

### Phase 3: Diagnostics

**Goal**: Report syntax, semantic, and type errors to the editor.

#### 3.1 Create Diagnostics Module
- [x] Create `src/lsp/diagnostics.rs`
- [x] Define internal diagnostic type with severity, message, span, and optional fix suggestion

#### 3.2 Wire Parser Errors to Diagnostics
- [x] On document open/change, parse the document
- [x] Convert parser `Error` nodes to LSP `Diagnostic` objects
- [x] Map spans to LSP `Range` (line/character positions)
- [x] Publish diagnostics via `textDocument/publishDiagnostics`

#### 3.3 Implement Semantic Analysis
- [ ] Validate block structure (e.g., `expect` only inside `@test`)
- [ ] Check for unknown assertion names
- [ ] Validate assertion syntax (e.g., `exitcode` expects a number)
- [ ] Report semantic errors as diagnostics

#### 3.4 Implement Type Checking for Assertions
- [ ] Define expected argument types for each assertion
- [ ] Validate argument types match expectations
- [ ] Report type mismatches (e.g., `exitcode "foo"` instead of `exitcode 0`)

#### 3.5 Add Fix Suggestions
- [ ] Include human-readable fix suggestions in diagnostic messages
- [ ] Example: "Unknown assertion 'stout'. Did you mean 'stdout'?"
- [ ] Suggestions are text-only (no code actions in this version)

#### 3.6 Graceful Degradation for Unknown Syntax
- [ ] Skip unrecognized blocks without crashing
- [ ] Continue analyzing the rest of the file
- [ ] Optionally emit a warning for unrecognized syntax

**Criteria addressed**: SC#2 (syntax errors), SC#3 (semantic and type errors)

---

### Phase 4: Completion

**Goal**: Provide context-aware completions with snippet support.

#### 4.1 Create Completion Module
- [ ] Create `src/lsp/completion.rs`
- [ ] Handle `textDocument/completion` request
- [ ] Determine cursor context (top-level, inside block, after `expect`)

#### 4.2 Implement Keyword Completion
- [ ] Complete `@test`, `@setup` at top-level positions
- [ ] Complete `expect`, `run` inside test blocks
- [ ] Filter completions based on valid positions

#### 4.3 Implement Assertion Name Completion
- [ ] After `expect `, suggest assertion names: `stdout`, `stderr`, `exitcode`, `file`, `duration`
- [ ] Include brief descriptions in completion item documentation

#### 4.4 Add Snippet Support
- [ ] Use `InsertTextFormat::Snippet` for completions
- [ ] Define snippets with tab stops (`$1`, `$2`, etc.)
- [ ] Implement snippet templates:
  - `@test "$1" {\n  $2\n}`
  - `expect stdout {\n  $1\n}`
  - `expect exitcode $1`

#### 4.5 Context-Aware Snippet Adaptation
- [ ] Adjust snippets based on surrounding context
- [ ] Avoid inserting braces if already present
- [ ] Handle indentation correctly

#### 4.6 Shell Command Completion
- [ ] Create `src/lsp/shell.rs` with command knowledge
- [ ] Hardcode common commands: `ls`, `cd`, `cat`, `echo`, `grep`, `git`, `curl`, `npm`, `cargo`, etc.
- [ ] Scan PATH for available executables (cached on startup)
- [ ] Suggest shell commands inside `run` blocks or at command positions

**Criteria addressed**: SC#4 (completion for keywords, assertions, shell commands), SC#5 (snippets with tab stops)

---

### Phase 5: Additional Features

**Goal**: Implement hover, outline, formatting, and semantic tokens.

#### 5.1 Implement Hover
- [ ] Create `src/lsp/hover.rs`
- [ ] Handle `textDocument/hover` request
- [ ] Look up the symbol at the cursor position
- [ ] Return documentation for keywords (`@test`, `@setup`, `expect`)
- [ ] Return documentation for assertion names with usage examples
- [ ] Format hover content as Markdown

#### 5.2 Implement Document Symbols
- [ ] Create `src/lsp/symbols.rs`
- [ ] Handle `textDocument/documentSymbol` request
- [ ] Walk the AST and emit symbols:
  - `@test` blocks → `SymbolKind::Function` with test name
  - `@setup` blocks → `SymbolKind::Constructor`
  - Nested `expect` blocks → `SymbolKind::Property` (children of test)
- [ ] Return hierarchical `DocumentSymbol[]` for outline view

#### 5.3 Implement Basic Formatting
- [ ] Create `src/lsp/formatting.rs`
- [ ] Handle `textDocument/formatting` request
- [ ] Normalize indentation:
  - Top-level blocks at column 0
  - Block contents indented one level (2 or 4 spaces, configurable default)
- [ ] Normalize spacing around braces and keywords
- [ ] Preserve internal formatting of shell commands and multiline strings
- [ ] Return `TextEdit[]` with changes

#### 5.4 Implement Semantic Tokens
- [ ] Create `src/lsp/semantic_tokens.rs`
- [ ] Register token types and modifiers in server capabilities
- [ ] Handle `textDocument/semanticTokens/full` request
- [ ] Emit tokens for:
  - Keywords (`@test`, `@setup`, `expect`) → `keyword`
  - Test names → `string`
  - Assertion names → `function`
  - Shell commands → `macro`
  - Comments → `comment`
- [ ] Encode tokens in LSP delta format

**Criteria addressed**: SC#6 (hover documentation), SC#7 (outline view), SC#8 (formatting)

---

### Phase 6: Polish and Multi-Editor Support

**Goal**: Ensure the LSP works across target editors and is well-documented.

#### 6.1 VS Code Integration
- [ ] Update `editors/vscode/package.json` with LSP client configuration
- [ ] Specify `hone lsp` as the server command
- [ ] Configure document selector for `.hone` files
- [ ] Test all features in VS Code
- [ ] Verify TextMate grammar fallback works

#### 6.2 Neovim Integration
- [ ] Write example `nvim-lspconfig` configuration
- [ ] Test with `nvim-lspconfig` setup
- [ ] Verify diagnostics, completion, hover, and outline work
- [ ] Document setup in README

#### 6.3 Helix Integration
- [ ] Add Hone configuration to `languages.toml` example
- [ ] Test all LSP features in Helix
- [ ] Document setup in README

#### 6.4 TextMate Grammar
- [ ] Verify `syntaxes/hone.tmlanguage.json` provides good baseline highlighting
- [ ] Ensure it covers all keywords, assertions, strings, and comments
- [ ] Test in editors without semantic token support

#### 6.5 Documentation
- [ ] Document LSP features in main README
- [ ] Add `docs/LSP_USAGE.md` with editor setup guides
- [ ] Document logging location and debugging tips
- [ ] Add troubleshooting section for common issues

#### 6.6 Testing Suite
- [ ] Write unit tests for each LSP handler
- [ ] Create mock LSP client test harness
- [ ] Test complete request/response cycles
- [ ] Test error handling (invalid requests, malformed documents)
- [ ] Achieve >80% code coverage for `src/lsp/`

**Criteria addressed**: SC#9 (works in VS Code, Neovim, Helix), SC#10 (all tests pass)

---

### Milestone Summary

| Milestone | Deliverable | Success Criteria |
|-----------|-------------|------------------|
| M1: Parser | Fault-tolerant parser with error nodes | SC#2 |
| M2: Server | Working `hone lsp` with lifecycle | SC#1 |
| M3: Diagnostics | Syntax + semantic + type errors | SC#2, SC#3 |
| M4: Completion | Keywords, assertions, snippets, shell | SC#4, SC#5 |
| M5: Features | Hover, outline, formatting, tokens | SC#6, SC#7, SC#8 |
| M6: Polish | Multi-editor support, docs, tests | SC#9, SC#10 |
