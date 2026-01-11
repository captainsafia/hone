# hone — CLI Integration Test Runner (v1 Specification)

## Overview

**hone** is a CLI integration testing tool for command-line applications, inspired by Hurl.

Tests are written in a **line-oriented DSL** that:

* executes real shell commands,
* runs them inside a persistent shell session,
* captures stdout and stderr separately,
* and asserts on output, filesystem effects, exit codes, and timing.

The runner is implemented in **Rust** and executes commands in a **PTY-backed shell** to preserve terminal-realistic behavior.

Windows is out of scope for v1.

---

## Design Principles

* Deterministic over clever
* Shell-realistic execution
* Minimal DSL, no control flow
* Fail fast, CI-friendly output
* Explicit over implicit (especially for naming and scoping)

---

## Execution Model

### Shell Session

* hone starts **one persistent shell session per DSL file**.
* The shell runs inside a **PTY** using `Bun.Terminal`.
* The shell is started in a **clean environment**:

  * user rc files are skipped when possible,
  * prompt output is suppressed (`PS1=`),
  * environment variables are explicitly constructed.

* **Shell initialization**: hone waits for the first shell prompt before executing tests, ensuring the shell is fully initialized.

All `RUN` commands in the file execute sequentially in this same shell session and share:

* working directory
* filesystem state
* shell state (e.g. `cd` persists)

### Shell Compatibility

* The runner requires shells that support:
  * `PS1` configuration for prompt suppression
  * exit code capture via `$?`
  * stderr redirection to files
  * `printf` with format codes
* If a shell does not meet these requirements, hone will **fail fast** with an error message.
* Recommended shells: bash, zsh (fish support if it meets requirements)

---

## Output Capture Model

For each `RUN`, hone captures:

* `stdout`: output written to the PTY (terminal output)
  * Available as both **stripped** (ANSI codes removed, default) and **raw** (with ANSI codes)
  * Selectors: `stdout` (ANSI-stripped), `stdout.raw` (with ANSI codes)
* `stderr`: output redirected to a temp file in `.hone/runs/<timestamp>-<run-id>/stderr.txt`
  * Preserved after test execution for debugging
  * ANSI codes handling TBD based on stderr PTY handling
* `exit_code`: shell exit code for the command
* `duration_ms`: wall-clock duration of the command (millisecond precision)

stdout and stderr are **captured separately**.

Ordering between stdout and stderr is **not preserved** and cannot be asserted on.

### ANSI Escape Codes

* **Dual capture mode**: hone captures both raw PTY output (with ANSI codes) and a stripped version (ANSI codes removed)
* Default selector `stdout` refers to the **stripped** version
* Use `stdout.raw` to assert on output including ANSI escape codes and colors
* This allows testing colored CLI output when needed while keeping most assertions simple

### Interactive Commands

* stdin is **closed** (`/dev/null`) for all commands
* Interactive commands that require stdin will fail or use default behavior
* This ensures deterministic execution and prevents tests from hanging on input prompts

---

## Sentinel Protocol (Command Framing)

Because commands run inside a persistent shell, hone uses a **sentinel protocol** to detect command completion.

### Sentinel Format

Sentinel line (written to stdout/PTY):

```
__HONE__<US><RUN_ID><US><EXIT_CODE><US><END_TS_MS>\n
```

Where:

* `<US>` is ASCII Unit Separator (`\x1f`)
* `<RUN_ID>` is a **composite identifier**: `<filename>-<testname>-<runname>`
  * Example: `init.hone-basic-test-build-step`
  * For unnamed RUNs, use sequential number: `init.hone-basic-test-1`
  * Makes debugging and log correlation straightforward
* `<EXIT_CODE>` is the shell exit code
* `<END_TS_MS>` is epoch milliseconds

### Command Timeout

* Default timeout per RUN: **30 seconds** (configurable via pragma)
* If a RUN does not produce its sentinel within the timeout, the test fails
* Pragma: `#! timeout: 60s` sets timeout for all RUNs in the file
* Prevents tests from hanging indefinitely in CI

### Per-RUN Shell Wrapper

For each `RUN`, hone writes the following to the shell:

```sh
: > "<STDERR_PATH>"
{ <USER_COMMAND> ; } 2> "<STDERR_PATH>"
HONE_EC=$?
printf "__HONE__\037<RUN_ID>\037%d\037%s\n" "$HONE_EC" "$(date +%s%3N)"
```

Rules:

* stderr is redirected to a **fresh temp file per RUN**
* sentinel is always written to stdout
* a RUN is considered complete when its sentinel is observed
* any output after the sentinel belongs to subsequent commands

---

## File Format (DSL)

### Lexical Rules

* Line-oriented; one statement per line
* Empty lines allowed
* Lines starting with `#` are comments
* Lines starting with `#!` at the top of the file are **pragmas**
* No inline comments in v1

### String Literals

* **Single quotes** (`'...'`): literal strings, no escape sequences
  * Example: `'hello\nworld'` contains literal backslash and n
* **Double quotes** (`"..."`): strings with escape sequences
  * Supported escapes: `\n` (newline), `\t` (tab), `\"` (quote), `\\` (backslash)
  * Example: `"hello\nworld"` contains a newline character
* Multi-line strings: **not supported in v1**
  * Use `\n` escapes in double-quoted strings instead

---

## File-Level Pragmas

Pragmas must appear at the top of the file.

```
#! shell: /bin/zsh
#! env: PATH=/custom/bin
#! env: FOO=bar
#! timeout: 60s
```

Supported pragmas:

* `shell:` — path to shell executable (default: `$SHELL`)
* `env:` — environment variable assignment (may appear multiple times)
  * Values are **literal** - no variable expansion or substitution
  * Example: `#! env: PATH=/custom/bin` sets PATH to exactly `/custom/bin`
* `timeout:` — timeout for all RUN commands in this file
  * Format: `<number>s` or `<number>ms`
  * Default: `30s`

### Pragma Handling

* Unknown pragmas generate a **warning** but do not cause failure
* This allows forward compatibility with future pragma additions
* Warnings are logged to stderr during test execution

---

## Tests

A DSL file contains zero or more named tests.

```
TEST "init works"
TEST "build and deploy"
```

### TEST Block Rules

* `TEST` blocks are **optional** - a file can contain bare `RUN` and `ASSERT` statements
  * Bare statements (without a TEST block) are treated as a single implicit test
* Test names must be quoted and can contain:
  * Alphanumeric characters
  * Spaces
  * Dashes and underscores
  * **No other special characters** (no quotes, symbols, etc.)
* Tests execute **in order** within a file
* All tests in a file share the **same shell session**
  * Shared working directory and filesystem state
  * Shared shell state (functions, aliases, variables from pragmas)
* Test-level `ENV` variables are **explicitly unset** between TEST blocks
  * hone runs `unset VAR1 VAR2...` before each new TEST
  * Ensures test-level environment isolation within the shared shell

---

## Statements

### ENV (test-level)

```
ENV KEY=value
ENV PATH=/custom/path
```

* Applies only within the current TEST block (or implicit test if no TEST blocks)
* Overrides file-level env from pragmas
* Values are **literal** - no variable expansion
  * `ENV PATH=$PATH:/custom` sets PATH to the literal string `$PATH:/custom`
* Variables are **explicitly unset** between TEST blocks to ensure isolation

---

### RUN

```
RUN <command>
RUN <name>: <command>
```

Examples:

```
RUN mycli init
RUN build: mycli build
RUN echo "test output"
```

Rules:

* One command per RUN
* Optional `name:` allows later assertions to reference this step
  * Names must be **unique across the entire file** (not just within TEST)
  * Duplicate names cause a parse error
* Commands are passed verbatim to the shell
* Non-zero exit codes **do not** automatically fail the test
  * Tests only fail if an `ASSERT` explicitly checks and fails
  * This allows testing error conditions and commands expected to fail

---

### ASSERT

Assertions apply to:

* the **most recent RUN**, or
* a **named RUN** via `<name>.`

```
ASSERT <expression>
ASSERT <name>.<expression>
```

---

## Assertion Types

### Output Assertions

```
ASSERT stdout contains "text"
ASSERT stderr matches /error/i
ASSERT stdout == "exact text"
ASSERT stdout.raw contains "\x1b[32m"
```

Selectors:

* `stdout` — ANSI-stripped output (default)
* `stdout.raw` — output with ANSI escape codes preserved
* `stderr` — error output (ANSI handling TBD)

Predicates:

* `contains <string>` — substring search (literal, not regex)
  * The string is treated as literal text
  * Regex special characters like `.` `*` `[` are treated as literal
* `matches <regex>` — regular expression match
  * Format: `/pattern/flags`
  * Supports JavaScript regex syntax and flags (i, g, m, etc.)
  * Example: `/error.*line \d+/i` for case-insensitive match
  * Regex validation happens at **assertion execution time**, not parse time
* `== <string>` — exact equality
* `!= <string>` — inequality

---

### Exit Code Assertions

```
ASSERT exit_code == 0
ASSERT build.exit_code != 127
```

Operators:

* `==`
* `!=`

---

### Timing Assertions

Wall-clock duration of a RUN.

```
ASSERT duration < 200ms
ASSERT build.duration <= 1.5s
```

Units:

* `ms`
* `s`

Operators:

* `==`, `!=`, `<`, `<=`, `>`, `>=`

Timing assertions are **strict**; any tolerance must be expressed explicitly.

---

### Filesystem Assertions

```
ASSERT file "out.txt" exists
ASSERT file "out.txt" contains "OK"
ASSERT file "out.txt" matches /OK:\s+\d+/
ASSERT file "out.txt" == "exact contents\n"
```

#### File Path Handling

* Paths are **relative to the current working directory** of the shell
  * If a test runs `RUN cd /tmp`, subsequent file assertions use `/tmp` as the base
  * hone tracks the shell's cwd state via directory changes
* File path comparisons require **exact casing**
  * On case-insensitive filesystems (macOS), hone errors if file exists with different casing
  * Forces portable tests that work consistently across platforms
  * Example: If `Output.txt` exists, `ASSERT file "output.txt" exists` will error with a casing mismatch warning

#### File Content Comparison

* Whitespace handling for `contains`, `==`, `!=`:
  * Trailing whitespace is **trimmed** from each line before comparison
  * Line endings are **normalized** to `\n` (treats `\r\n` and `\n` as equivalent)
  * This makes tests more robust across platforms and editors
* Empty files (`0 bytes`) are valid and can be asserted on
  * Example: `ASSERT file "empty.txt" == ""` passes for a 0-byte file

#### Predicates

* `exists` — file exists at path
* `contains <string>` — file contains substring (literal, not regex)
* `matches <regex>` — file content matches regex pattern
* `== <string>` — exact content equality (after whitespace normalization)
* `!= <string>` — content inequality

---

## Failure Behavior

### Within a Test File

* hone stops on the **first failure** within a test file
* A failure is:
  * A failed ASSERT statement
  * A parse error in the DSL
  * A timeout waiting for command completion
  * **NOT** a non-zero exit code from a RUN (unless explicitly asserted)
* Once a failure occurs, the remaining statements in the file are skipped

### Across Multiple Files

When running multiple test files via glob pattern:

* All files are run **sequentially**
* hone **continues** to run all files even if some fail
* All failures are **collected** and reported at the end
* Final exit code is non-zero if any file failed

### Exit Codes

* **0**: All tests passed
* **1**: One or more tests failed (any type of failure)
* Simple binary pass/fail — no distinction between failure types

### Error Output

Default error output (without `--verbose`):

```
FAIL example.hone:42 :: "init works"
RUN: mycli init
ASSERT stdout contains "created"
Expected: stdout to contain "created"
Actual stdout:
  Initialization started
  Setup complete
```

Error output includes:

* File location in `filename:line` format (parseable by editors/CI)
* Test name (if in a TEST block)
* The RUN command that was executed
* The failed assertion
* The actual output received

With `--verbose` flag: full stdout/stderr dumps included in output

---

## Runner CLI

```
hone run <path | glob>
```

### Arguments

* `<path | glob>` — test file path or glob pattern
  * Glob patterns use **shell-style** expansion (non-recursive by default)
  * Examples:
    * `hone run test.hone` — single file
    * `hone run *.hone` — all `.hone` files in current directory
    * `hone run tests/**/*.hone` — recursive search in `tests/` directory
  * When multiple files match, they run sequentially with failure collection

### Flags

* `--shell <path>` — override shell executable
  * Overrides `#! shell:` pragma and `$SHELL` environment variable
* `--verbose` — include full stdout/stderr dumps on failure
  * Default output shows only essential context
  * Verbose mode includes complete output for debugging

### Progress Output

During test execution, hone prints:

* Test file being executed
* Dots (`.`) or checkmarks (`✓`) for each RUN command as it completes
* Provides visual feedback without verbose output
* Example:
  ```
  Running test.hone
  ....✓✓✓✓
  PASS test.hone (8 assertions)
  ```

---

## Clean Environment Rules

hone starts the shell with:

* **No user rc files** (when supported by shell)
  * bash: `--norc --noprofile`
  * zsh: `--no-rcs`
* **Empty prompt** (`PS1=`) to suppress prompt output
* **stdin closed** (`/dev/null`) to prevent interactive hangs

Environment is composed from:

1. **Minimal base** (e.g., PATH, HOME if required by shell)
2. **File-level pragmas** (`#! env:` statements)
3. **Test-level ENV statements** (scoped to current TEST, unset between tests)

Environment variables use **literal values** with no expansion at any level.

---

## Artifact Management

### .hone Directory

hone creates a `.hone/` directory in the current working directory for test artifacts:

```
.hone/
  runs/
    <timestamp>-<run-id>/
      stderr.txt
      metadata.json (optional)
```

Structure:

* **`.hone/runs/<timestamp>-<run-id>/`** — directory per RUN execution
  * `<timestamp>`: ISO 8601 format (e.g., `2025-01-04T10-30-45`)
  * `<run-id>`: composite ID (e.g., `test.hone-build-step`)
* **`stderr.txt`** — captured stderr for the RUN
* Artifacts are **preserved** after test execution for debugging
* Users can clean up manually or ignore `.hone/` in version control

---

## Known Limitations (v1)

* Background processes that write after command completion are undefined behavior
* No control flow (loops, conditionals, retries)
* No ordering assertions between stdout and stderr
* Unix-only (Linux and macOS)
* No snapshot testing
* No multi-line string literals (use `\n` escapes)
* No variable expansion in ENV values (literal only)
* No setup/teardown hooks (use RUN statements instead)
* No validation/dry-run mode (coming in future version)
* No output size limits (unbounded memory usage for huge outputs)
* No built-in variables or constants in assertions

---

## Non-Goals

* Windows support
* Parallel execution across test files
* Parallel execution within a test file
* Structured output parsing (JSONPath, XML, etc.)
* Snapshot management UI
* Interactive test authoring/debugging
* IDE integrations (in v1)

---

## Implementation Notes (for Agents)

### Parser

* **Hand-written, line-based parser** — no parser generators
* **AST must preserve line numbers** for error diagnostics
* Line number format: `filename:line` for editor/CI compatibility
* Parse errors should be clear and point to exact line

### PTY and Shell Management

* **PTY allocation** via `Bun.Terminal` (or equivalent Bun API)
* **Wait for first prompt** before executing tests
  * Detect prompt readiness (shell-specific detection)
  * Ensures shell has fully initialized (100ms+ for some shells)
* One shell session per file, isolated across files
* Shell compatibility check on startup (fail fast if unsupported)

### Sentinel Protocol

* **Strict sentinel parsing** — defensive against output contamination
* Sentinel format: `__HONE__\x1f<run-id>\x1f<exit-code>\x1f<timestamp>\n`
* Must handle:
  * User output containing similar patterns
  * Incomplete sentinel writes
  * Multiple sentinels in buffer
* Timeout enforcement: 30s default, configurable via pragma

### ANSI Code Handling

* **Dual capture**: both raw and stripped versions of stdout
* Use ANSI parsing library (e.g., `ansi-regex` or similar)
* Strip codes for default `stdout` selector
* Preserve raw output for `stdout.raw` selector

### File Path and cwd Tracking

* Track shell working directory changes via `cd` commands
* **Relative paths** in file assertions use current shell cwd
* **Exact casing** validation on file operations
  * On case-insensitive filesystems, error if casing doesn't match
* Whitespace normalization for file content assertions

### String Handling

* **Single quotes**: literal (no escaping)
* **Double quotes**: support `\n`, `\t`, `\"`, `\\`
* Regex format: `/pattern/flags` (JavaScript regex)
* `contains` uses literal substring search (not regex)
* `matches` uses regex with validation at assertion time

### Artifact Management

* Create `.hone/runs/<timestamp>-<run-id>/` per RUN
* Store stderr in `stderr.txt`
* Preserve artifacts across runs (no eager cleanup)
* Users manage `.hone/` cleanup manually

### Error Handling

* **Fail fast** within a file on first assertion failure
* **Collect all failures** across multiple files
* Error format: `filename:line :: "test name"`
* Include RUN command, assertion, expected vs actual in error output
* Non-zero exit codes from RUNs do **not** fail tests automatically

### Progress and Output

* Print filename when starting each test file
* Print dots/checkmarks for each RUN as it completes
* Default output: minimal, CI-friendly
* `--verbose`: full stdout/stderr dumps

### Pragma and ENV Handling

* Unknown pragmas: **warn** but continue (forward compatibility)
* ENV values: **literal** (no variable expansion)
* Test-level ENV: **explicitly unset** between TEST blocks
  * Track which vars were set in each TEST
  * Run `unset VAR1 VAR2...` before starting next TEST

### Multi-File Execution

* **Complete isolation** across files (independent shells)
* Sequential execution with failure collection
* Final exit code: 0 if all pass, 1 if any fail

---

## Implementation Architecture

This section documents the technical implementation approach.

### Technology Stack

* **Language**: Rust with Tokio async runtime
* **CLI Framework**: clap v4 with derive macros
  * Make `run` the default command: `hone test.hone` and `hone run test.hone` both work
  * Leaves room for future commands (`hone validate`, `hone init`, etc.)
* **Terminal Colors**: owo-colors for colored output
  * Automatically detects TTY support
  * Colored output in interactive terminals
  * Plain output in CI environments
* **ANSI Stripping**: strip-ansi-escapes crate for removing escape codes
* **Error Handling**: anyhow for error propagation, thiserror for custom error types

### Project Structure

```
hone/
├── src/
│   ├── main.rs                # Entry point, clap setup, arg parsing
│   ├── lib.rs                 # Library exports
│   ├── parser/
│   │   ├── mod.rs             # Parser module exports
│   │   ├── parser.rs          # Main parser entry point
│   │   ├── lexer.rs           # Line-based tokenization
│   │   ├── ast.rs             # AST type definitions (enums)
│   │   └── errors.rs          # Parse error collection and reporting
│   ├── runner/
│   │   ├── mod.rs             # Runner module exports
│   │   ├── executor.rs        # Test runner orchestration
│   │   ├── shell.rs           # Shell session management
│   │   ├── sentinel.rs        # Sentinel protocol implementation
│   │   └── reporter.rs        # Progress/error reporting interface
│   ├── assertions/
│   │   ├── mod.rs             # Assertions module exports
│   │   ├── output.rs          # stdout/stderr assertions
│   │   ├── filesystem.rs      # file assertions
│   │   ├── timing.rs          # duration assertions
│   │   └── exitcode.rs        # exit_code assertions
│   └── utils/
│       ├── mod.rs             # Utils module exports
│       └── ansi.rs            # ANSI code stripping utilities
├── tests/integration/         # .hone integration test files
├── examples/                  # .hone example files (living documentation)
├── .github/
│   └── workflows/
│       ├── ci.yml             # Multi-OS testing, security audits
│       └── release.yml        # Cross-compilation, releases
├── Cargo.toml
├── Cargo.lock
└── README.md
```

### AST Design

Use **Rust enums** for exhaustive pattern matching:

```rust
pub enum ASTNode {
    Test(TestNode),
    Run(RunNode),
    Assert(AssertNode),
    Env(EnvNode),
    Pragma(PragmaNode),
    Comment(CommentNode),
}

pub struct TestNode {
    pub name: String,
    pub line: usize,
}

pub struct RunNode {
    pub name: Option<String>,
    pub command: String,
    pub line: usize,
}

// ... etc
```

* All nodes include `line: usize` for error reporting
* Parser validates entire file and **collects all parse errors** before failing
* Errors formatted as `filename:line` for editor/CI integration

### Parser Implementation

* **Full file parse to AST** before execution
* Two-phase approach:
  1. **Parse phase**: Tokenize and build AST, collect all syntax errors
  2. **Validation phase**: Semantic checks (duplicate names, orphaned assertions)
* Line-based lexer (no complex grammar)
* Parse errors do not stop parsing - collect all errors and report together
* Single vs double quote distinction for string escapes

### PTY and Shell Management

* Uses Tokio's async process management with stdin/stdout/stderr pipes
* One `ShellSession` struct per test file
* Shell detection strategy:
  * **Allowlist**: bash, zsh (fast path)
  * **Probe**: unknown shells tested for capabilities (PS1, $?, printf)
  * Fail fast if shell doesn't meet requirements
* Wait for first prompt using shell-specific detection
* Resource cleanup via **Drop trait**:
  * Implements `Drop` for automatic cleanup
  * SIGINT/SIGTERM handlers for graceful shutdown
  * Ensures processes and temp files are cleaned up

### Sentinel Protocol

* **Hard-coded format**: `__HONE__\x1f<run-id>\x1f<exit-code>\x1f<timestamp>\n`
* Strict parsing with defensive checks:
  * Partial sentinel detection
  * User output containing similar patterns
  * Buffer overflow handling
* 30-second timeout enforcement (configurable via pragma)
* RUN_ID format: `<filename>-<testname>-<runname>` (or sequential number if unnamed)

### Working Directory Tracking

* **Query shell before each file assertion**: run `pwd` to get current directory
* More reliable than trying to track `cd` commands
* Slower but guarantees accuracy even with complex shell scripts
* File paths in assertions are relative to current shell cwd

### Assertion Implementation

Separate files by assertion type for maintainability:

* `output.ts`: stdout/stderr with dual capture (raw/stripped ANSI)
* `filesystem.ts`: file existence, content matching with security validation
* `timing.ts`: duration comparisons with millisecond precision
* `exitcode.ts`: exit code assertions

Security hardening:

* **Path traversal validation** for all file operations
* Reject paths containing `..` outside the test working directory
* Prevents malicious tests from accessing sensitive files

### Reporter Interface

Abstract progress/error reporting for clean separation:

```rust
pub trait Reporter {
    fn on_file_start(&self, filename: &str);
    fn on_run_complete(&self, run_id: &str, success: bool);
    fn on_assertion_pass(&self);
    fn on_parse_errors(&self, errors: &[ParseErrorDetail]);
    fn on_warning(&self, message: &str);
    fn on_summary(&self, results: &TestResults);
}
```

Default implementation:

* Prints filename when starting each file
* Dots (`.`) or checkmarks (`✓`) for each RUN
* Failure details with context (RUN command, assertion, actual output)
* Final summary with pass/fail counts

### Multi-File Execution

* **Parallel parsing**: parse all matched files concurrently using `futures::future::join_all()`
* **Sequential execution**: run test files one at a time, isolated shells
* **Failure collection**: continue running all files even if some fail
* Final exit code: 0 for all pass, 1 if any fail

### Artifact Management

* Create `.hone/runs/<timestamp>-<run-id>/` per RUN
* Timestamp format: `20250104-103045` (sortable, compact)
* Store `stderr.txt` in each run directory
* No automatic cleanup - users manage `.hone/` directory manually
* Recommend adding `.hone/` to `.gitignore`

### Resource Cleanup

Cleanup via Drop trait:

```rust
impl Drop for ShellSession {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }
    }
}
```

Plus process signal handlers for graceful shutdown on SIGINT/SIGTERM.

### Distribution

* **Compiled standalone binaries** for all platforms:
  * Linux: x64, ARM64
  * macOS: x64 (Intel), ARM64 (Apple Silicon)
* **Statically linked Rust binaries**:
  * No runtime dependencies required
  * Single binary with all code compiled in
  * Cross-compiled using `cross` tool for different architectures
* **GitHub Actions CI/CD**:
  * Multi-OS testing matrix (Linux, macOS)
  * Security audits with cargo-audit
  * Automated releases on git tags
  * Cross-compilation for all target platforms

### Testing Strategy

* **Dogfooding**: Use hone to test itself
  * Write `.hone` test files for integration testing
  * Proves the tool works for real-world use cases
* **Cargo test** for unit tests:
  * Parser tests (valid/invalid syntax)
  * Assertion logic tests
  * Utility function tests
* Test structure:
  * Unit tests inline with code using `#[test]` annotations
  * `tests/integration/` - `.hone` files testing the CLI end-to-end
  * `examples/` - `.hone` files serving as documentation and smoke tests

### Configuration

* **Pragma-only** - no global config file
* All configuration in `.hone` file pragmas
* No `~/.config/hone/` directory needed
* Keeps tests self-contained and portable
* CI-friendly: no hidden config dependencies

### Error Handling

* **Centralized error handling with anyhow**:
  * Error propagation using `?` operator
  * Context added at each layer for debugging
  * Graceful shutdown with cleanup
* Error output with owo-colors:
  * Red for failures
  * Yellow for warnings
  * Dimmed for context
* Parser errors collected and reported together
* Runtime errors include full context (file, line, test name, command)

### Future Considerations (Not in v1)

* Library API extraction (design internals to support future export)
* Plugin system (structure code with future hooks in mind, but don't implement)
* Alternative output formats (JSON, TAP) - reporter interface makes this easy
* Validation/dry-run mode
* Watch mode for development