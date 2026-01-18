# Hone VS Code Extension

VS Code extension providing language support for [Hone](https://github.com/captainsafia/hone) test files.

## Features

### Syntax Highlighting
- Full syntax highlighting for `.hone` test files via TextMate grammar
- Semantic token support for enhanced highlighting

### Language Server Features
- **Diagnostics**: Real-time syntax, semantic, and type error checking
- **Completion**: Context-aware completions for keywords, assertions, and shell commands
- **Hover**: Documentation for keywords and assertions
- **Document Symbols**: Outline view showing test structure
- **Formatting**: Automatic indentation and spacing normalization
- **Snippets**: Tab-stop enabled snippets for common patterns

## Requirements

- VS Code 1.75.0 or higher
- Hone CLI installed and available in PATH (or configured via `hone.lsp.path`)

## Installation

1. Install the Hone CLI: see [Hone installation instructions](https://github.com/captainsafia/hone)
2. Install this extension from the VS Code marketplace or build from source

## Configuration

- `hone.lsp.path`: Path to the hone executable (default: `"hone"`)
- `hone.lsp.trace.server`: Enable LSP communication tracing for debugging (default: `"off"`)

## Usage

Open any `.hone` file and the language server will automatically start, providing diagnostics, completions, and other features.

## Development

To build the extension from source:

```bash
cd editors/vscode
npm install
npm run compile
```

To package:

```bash
npx vsce package
```

## License

See LICENSE file in the repository.
