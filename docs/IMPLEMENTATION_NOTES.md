# Implementation Notes

This document describes the implementation of Test Explorer integration for the Hone VS Code extension.

## Architecture

The extension is structured into several modules:

### Core Modules

1. **extension.ts** - Entry point that activates the extension and creates the test controller
2. **testController.ts** - Main orchestrator implementing VS Code Test Controller API
3. **testDiscovery.ts** - File discovery and parsing logic for `.hone` files
4. **testRunner.ts** - Test execution and terminal management
5. **hone.ts** - CLI interaction, binary discovery, and JSON parsing
6. **diagnostics.ts** - Diagnostic marker management for test failures
7. **codeLens.ts** - CodeLens provider for inline test actions

## Key Design Decisions

### Lazy Loading
Test files are discovered eagerly, but TEST blocks are only parsed when a file is expanded in the Test Explorer. This improves performance for large workspaces.

### Three-Level Hierarchy
- **Level 1**: Files (.hone files)
- **Level 2**: TEST blocks
- **Level 3**: RUN commands

### Test Execution Granularity
- **File level**: Runs all TEST blocks in the file
- **TEST level**: Runs a single TEST using `--test` filter
- **RUN level**: Runs the parent TEST block (RUN commands cannot be run in isolation)

### Terminal Management
A single "Hone Tests" terminal is created and reused for all test runs. The terminal is cleared before each run to provide a clean output view.

### JSON Output Dependency
The extension requires the hone CLI to support `--output-format json` for rich test results. If JSON output is not available, it falls back to basic exit code checking.

## API Usage

### Test Controller API
Uses VS Code's Test Controller API (stable since VS Code 1.59):
- `vscode.tests.createTestController()` - Creates the test controller
- `controller.resolveHandler` - Lazy loads test items
- `controller.createRunProfile()` - Handles test execution
- `TestItem` hierarchy for file/TEST/RUN structure

### File System Watcher
Monitors `.hone` files for changes:
- `onDidCreate` - Adds new test files
- `onDidChange` - Re-parses modified files
- `onDidDelete` - Removes test files from tree

### CodeLens Provider
Implements `vscode.CodeLensProvider` to show inline actions above TEST blocks.

### Diagnostics
Uses `vscode.languages.createDiagnosticCollection()` to mark failing assertions with error severity.

## Regex Patterns

### TEST Block Detection
```regex
/^TEST\s+"([^"]+)"/
```
Matches: `TEST "test name"`

### RUN Command Detection  
```regex
/^RUN(?:\s+(\w+):)?\s+(.+)$/
```
Matches:
- `RUN command` (unnamed)
- `RUN name: command` (named)

## Error Handling

### Missing Hone Binary
When the hone CLI is not found in PATH:
1. Show error notification
2. Offer to install automatically via curl script
3. Provide manual installation instructions link

### Malformed Files
If a `.hone` file cannot be parsed:
- Log error to console
- Return empty test block array
- File still appears in tree but with no children

### Test Execution Failures
- Parse JSON output if available
- Fall back to exit code checking
- Create diagnostic markers for assertion failures
- Show detailed error messages in test results

## Performance Considerations

### Lazy Parsing
Only parse file contents when needed, not during initial discovery.

### Regex-based Parsing
Simple regex patterns are fast and sufficient for discovery. Edge cases (multiline commands, complex escaping) are acceptable limitations.

### Single Terminal
Reusing one terminal avoids creating many terminal instances.

## Future Enhancements

Potential improvements not included in this implementation:

1. **Diff View**: Open VS Code diff editor for expected vs actual on test failure
2. **Coverage Integration**: Show test coverage if hone supports it
3. **Test Debugging**: Debug support with breakpoints
4. **Configuration Options**: Settings for exclusion patterns, custom test discovery
5. **Multi-root Workspace**: Enhanced support for multiple workspace folders
6. **Test Filtering**: UI for filtering tests by status or name
7. **Duration Display**: Show test duration inline in test tree labels
8. **Parallel Execution**: Run multiple tests concurrently

## Testing the Extension

To test the extension locally:

1. Open the extension directory in VS Code
2. Press F5 to launch Extension Development Host
3. Open a workspace with `.hone` files
4. View tests in Test Explorer sidebar
5. Run tests and verify:
   - Test tree populates correctly
   - Tests execute and show results
   - Diagnostics appear for failures
   - CodeLens actions work
   - Terminal shows output

## Known Limitations

1. **JSON Output Required**: Rich results require hone CLI with `--output-format json` support
2. **Regex Parsing**: Complex multiline commands may not parse correctly
3. **RUN Granularity**: RUN commands execute their parent TEST (cannot run individual RUNs)
4. **No Persistence**: Test results are cleared on VS Code reload
5. **PATH Only**: Binary must be in PATH (no custom path configuration)
