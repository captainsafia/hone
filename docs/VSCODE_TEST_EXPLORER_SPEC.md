# VS Code Test Explorer Integration Specification

This document specifies the integration of hone tests into VS Code's Test Explorer.

## Overview

The VS Code extension will be enhanced to support:
- Automatic discovery of `.hone` test files
- Display of tests in VS Code's Test Explorer with a three-level hierarchy
- Running tests at file, TEST block, or RUN command granularity
- Rich failure reporting with diffs and diagnostic markers
- CodeLens integration for running tests from the editor

## Prerequisites

Before starting extension development, the hone CLI must be extended with:

### CLI Changes Required

1. **Test filtering**: `--test <pattern>` flag
   - Supports exact name matching: `--test "my test name"`
   - Supports regex patterns: `--test "/pattern/"`
   - Runs only TEST blocks matching the pattern

2. **JSON output**: `--output-format json` flag
   - Verbose output including:
     - Pass/fail status per TEST and RUN
     - Test and run names
     - stdout/stderr content
     - Exit codes for each RUN
     - Assertion details (expected vs actual)
     - Timing per command
     - Line numbers in source file
     - Raw output when relevant

## Test Discovery

### File Discovery

- **Auto-discover** all `.hone` files in the workspace
- **Lazy parsing**: Find files eagerly on workspace open, but only parse TEST/RUN structure when files are expanded in Test Explorer
- **Watch events**: Refresh test tree only on file save (not during typing or on file create/delete until saved)

### Tree Hierarchy

Three-level structure:
```
📁 tests/
  📄 basic.hone
    🧪 echo test
      ▶ echo (named run)
    🧪 file creation
      ▶ echo "test content" > /tmp/...
      ▶ [no assertions for this run]
    🧪 error handling
      ▶ false
```

- **Level 1**: Files (organized by directory structure)
- **Level 2**: TEST blocks (by test name from `TEST "name"`)
- **Level 3**: RUN commands
  - Named runs: Display the name (e.g., `build` from `RUN build: make build`)
  - Unnamed runs: Display the command text

### Parser

Use **regex-based extraction** for parsing `.hone` files:
- Simple regex patterns to find `TEST "..."` blocks and `RUN ...` commands
- Fast and sufficient for discovery purposes
- Accept that edge cases (multiline commands, complex escaping) may not parse perfectly

## Test Execution

### Granularity

- **File level**: Run all TEST blocks in a file
- **TEST level**: Run a single TEST block using `--test` filter
- **RUN level**: Clicking run on a RUN command runs its parent TEST block

### Binary Location

- Locate `hone` via **PATH lookup only**
- If not found:
  - Show notification offering to install
  - If user accepts, run the curl install script: `curl https://i.captainsafia.sh/captainsafia/hone | sh`
  - Provide link to manual installation instructions

### Workspace Scope

- **Full workspace support**: Run all `.hone` files, run by folder, or run individual files/tests
- Tests can be run from any level in the hierarchy

### Terminal Output

- Run tests in an **integrated terminal**
- **Reuse single terminal**: One terminal named "Hone Tests", cleared and reused for each run
- Show real-time command output as tests execute

### Keyboard Shortcuts

Use **VS Code's standard test shortcuts**:
- `Ctrl+; Ctrl+C` / `Cmd+; Cmd+C`: Run test at cursor
- `Ctrl+; Ctrl+A` / `Cmd+; Cmd+A`: Run all tests
- Other standard test shortcuts as defined by VS Code

## Results Display

### Test States

- **Passed**: Green checkmark
- **Failed**: Red X with error details
- **Running**: Spinner animation
- **Not run**: No icon (initial state)

### Duration Display

- Show duration **inline after test name**: `my test (1.2s)`
- Duration shown in the Test Explorer tree for completed tests

### Failure Reporting

Three mechanisms for displaying failures:

1. **Mark RUN as failed**: The RUN node in the tree shows failure state
   - Details available in output panel when selected

2. **Inline diff view**:
   - Triggered when **clicking on a failed test** in the explorer
   - Opens VS Code diff editor showing expected vs actual values
   - For string comparisons, file content assertions, etc.

3. **Diagnostic markers**:
   - **Error severity (red)** squiggly underlines on failing ASSERT lines
   - Appears in the Problems panel as errors
   - Includes assertion failure message

### Result Persistence

- **Clear on reload**: Results are cleared when VS Code is reloaded
- No persistence across sessions
- Prevents showing stale results

## CodeLens Integration

Display CodeLens above each TEST block:

```
▶ Run Test | 🔍 View in Explorer
TEST "my test name"
```

- **Run Test**: Execute this TEST block
- **View in Explorer**: Reveal and select this test in the Test Explorer panel

## Shell Handling

- **Trust the pragma**: Use the `#! shell:` pragma if present in the file
- Otherwise, use hone's default shell behavior
- No warnings or UI for shell selection

## Extension Configuration

Minimal configuration required:

```jsonc
{
  // No required settings - extension auto-discovers and uses PATH
}
```

Optional settings may be added later for:
- Excluding certain directories from discovery
- Custom glob patterns for test files

## VS Code Version

- **Minimum**: `^1.75.0`
- Uses VS Code's Test Controller API (stable since 1.59)

## Implementation Phases

### Phase 1: CLI Changes
1. Implement `--test <pattern>` flag in hone CLI
2. Implement `--output-format json` flag with verbose output
3. Release updated hone CLI

### Phase 2: Basic Extension
1. Convert extension to TypeScript
2. Implement Test Controller registration
3. Implement lazy file discovery
4. Implement regex-based TEST/RUN parsing
5. Implement basic test running (file-level and TEST-level)

### Phase 3: Rich Features
1. Add JSON output parsing for detailed results
2. Implement diagnostic markers for failures
3. Implement diff view for assertion failures
4. Add CodeLens support
5. Add hone installation prompt

### Phase 4: Polish
1. Add duration display
2. Optimize performance for large workspaces
3. Add workspace-wide test runs
4. Testing and bug fixes

## File Structure

```
editors/vscode/
├── package.json           # Extension manifest (updated)
├── language-configuration.json
├── src/
│   ├── extension.ts       # Extension entry point
│   ├── testController.ts  # Test Controller implementation
│   ├── testDiscovery.ts   # File watching and parsing
│   ├── testRunner.ts      # Test execution
│   ├── diagnostics.ts     # Diagnostic markers
│   ├── codeLens.ts        # CodeLens provider
│   └── hone.ts            # Hone CLI interaction
└── syntaxes/
    └── hone.tmlanguage.json
```

## JSON Output Format

The `--output-format json` flag should produce output like:

```json
{
  "file": "tests/basic.hone",
  "shell": "/bin/bash",
  "tests": [
    {
      "name": "echo test",
      "line": 5,
      "status": "passed",
      "duration_ms": 120,
      "runs": [
        {
          "name": "echo",
          "command": "echo \"hello world\"",
          "line": 7,
          "status": "passed",
          "duration_ms": 45,
          "exit_code": 0,
          "stdout": "hello world\n",
          "stderr": "",
          "assertions": [
            {
              "line": 8,
              "expression": "exit_code == 0",
              "status": "passed"
            },
            {
              "line": 9,
              "expression": "stdout contains \"hello\"",
              "status": "passed"
            }
          ]
        }
      ]
    },
    {
      "name": "failing test",
      "line": 15,
      "status": "failed",
      "duration_ms": 89,
      "runs": [
        {
          "name": null,
          "command": "echo \"wrong output\"",
          "line": 17,
          "status": "failed",
          "duration_ms": 32,
          "exit_code": 0,
          "stdout": "wrong output\n",
          "stderr": "",
          "assertions": [
            {
              "line": 18,
              "expression": "stdout == \"expected output\"",
              "status": "failed",
              "expected": "expected output",
              "actual": "wrong output\n"
            }
          ]
        }
      ]
    }
  ],
  "summary": {
    "total_tests": 2,
    "passed": 1,
    "failed": 1,
    "duration_ms": 209
  }
}
```
