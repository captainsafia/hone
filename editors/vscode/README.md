# Hone VS Code Extension

VS Code extension providing syntax highlighting and Test Explorer integration for [Hone](https://github.com/captainsafia/hone), a CLI integration test runner.

## Features

### Syntax Highlighting
- Full syntax highlighting for `.hone` test files
- Support for TEST blocks, RUN commands, ASSERT statements, and pragmas

### Test Explorer Integration
- Automatic discovery of `.hone` test files in your workspace
- Three-level test hierarchy: File → TEST block → RUN command
- Run tests at file, TEST block, or RUN command level
- Real-time test execution in integrated terminal
- Rich failure reporting with:
  - Diagnostic markers on failing assertions
  - Detailed error messages with expected vs actual values
  - Integration with VS Code's Problems panel

### CodeLens
- Inline "Run Test" actions above each TEST block
- "View in Explorer" links to reveal tests in Test Explorer
- Quick access to test execution from your editor

### Keyboard Shortcuts
Standard VS Code test shortcuts work automatically:
- `Ctrl+; Ctrl+C` (Cmd on Mac): Run test at cursor
- `Ctrl+; Ctrl+A` (Cmd on Mac): Run all tests

## Requirements

- VS Code 1.75.0 or higher
- Hone CLI installed and available in PATH

The extension will prompt you to install Hone if it's not found.

## Usage

1. Open a workspace containing `.hone` test files
2. Tests will automatically appear in the Test Explorer sidebar
3. Click the play button next to any file, TEST, or RUN to execute it
4. View test results in the Test Explorer and Problems panel

## Features in Detail

### Lazy Loading
Test files are discovered immediately, but TEST blocks are only parsed when you expand a file in the Test Explorer. This keeps the extension fast even in large workspaces.

### Terminal Integration
Tests run in a single "Hone Tests" terminal that's reused for each run, showing real-time output as tests execute.

### Diagnostics
Failed assertions are marked with red squiggly underlines in the editor, showing:
- Which assertion failed
- Expected vs actual values
- Full error context

### JSON Output Support
The extension uses `hone --output-format json` when available for rich test results. Falls back to basic exit code checking if JSON output is not supported.

## Development

### Building from Source
```bash
cd editors/vscode
npm install
npm run compile
```

### Packaging
```bash
npm run vscode:prepublish
vsce package
```

## License

See LICENSE file in the repository.
