# Hone LSP for Helix

This directory contains the Helix configuration for the Hone language server.

## Prerequisites

- Helix 23.10 or later
- The `hone` binary in your PATH

## Setup

Helix uses a `languages.toml` file to configure language servers. The configuration file location depends on your platform:

- Linux/macOS: `~/.config/helix/languages.toml`
- Windows: `%AppData%\helix\languages.toml`

### Option 1: Add to Your Configuration

Add the following to your `languages.toml`:

```toml
[[language]]
name = "hone"
scope = "source.hone"
injection-regex = "hone"
file-types = ["hone"]
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
language-servers = ["hone-lsp"]

[language-server.hone-lsp]
command = "hone"
args = ["lsp"]
```

### Option 2: Use the Provided Configuration

You can copy the `languages.toml` file from this directory to your Helix config directory:

```sh
# Linux/macOS
cp languages.toml ~/.config/helix/languages.toml

# Or append to existing config
cat languages.toml >> ~/.config/helix/languages.toml

# Windows (PowerShell)
Copy-Item languages.toml $env:AppData\helix\languages.toml
```

### Verify Installation

1. Verify `hone` is installed and in your PATH:
   ```sh
   which hone
   hone --version
   ```

2. Test the LSP directly:
   ```sh
   hone lsp
   ```
   (The server should start and wait for input. Press Ctrl+C to exit.)

3. Open a `.hone` file in Helix and check the language server status:
   ```
   :log-open
   ```
   Look for Hone LSP initialization messages.

## Features

The Hone language server provides:

- **Diagnostics**: Real-time syntax, semantic, and type checking errors
- **Completion**: Context-aware completions for:
  - Keywords (`@test`, `@setup`, `expect`, `run`)
  - Assertions (`stdout`, `stderr`, `exitcode`, `file`, `duration`)
  - Shell commands (common commands + executables in PATH)
  - Code snippets with tab stops
- **Hover**: Documentation for keywords and assertions (press `Space+k` by default)
- **Document Symbols**: Outline view showing test structure (press `Space+s` by default)
- **Formatting**: Normalize indentation and spacing (press `Space+f` by default)
- **Semantic Highlighting**: Enhanced syntax highlighting

## Key Bindings

Default Helix key bindings for LSP features:

| Action | Key Binding |
|--------|-------------|
| Code completion | `Ctrl+x` (insert mode) |
| Hover documentation | `Space+k` |
| Goto next diagnostic | `]d` |
| Goto previous diagnostic | `[d` |
| Format document | `Space+f` |
| Document symbols (outline) | `Space+s` |
| Rename symbol | `Space+r` |

You can customize these in your `config.toml` file.

## Usage Examples

### Diagnostics

Helix will automatically show diagnostics as you type. Errors appear inline with red underlines. Navigate through diagnostics with `]d` and `[d`.

### Completion

1. Type `@test` and press `Ctrl+x` to see completions
2. Select a completion and press `Enter`
3. Use `Tab` to jump between snippet placeholders

After typing `expect ` (with trailing space), completions for assertions automatically appear.

### Hover Documentation

Move your cursor over a keyword (like `@test` or `expect`) and press `Space+k` to see documentation.

### Document Symbols

Press `Space+s` to open the document symbol picker. This shows an outline of all tests and setup blocks in the current file.

### Formatting

Press `Space+f` to format the current document. This normalizes indentation and spacing while preserving shell command content.

## Troubleshooting

### Server not starting

1. Verify `hone` is installed and in your PATH:
   ```sh
   which hone
   hone --version
   ```

2. Check that your `languages.toml` is correctly configured:
   ```sh
   # Linux/macOS
   cat ~/.config/helix/languages.toml
   ```

3. Check LSP logs in Helix:
   - Open Helix
   - Run `:log-open` to view logs
   - Look for Hone LSP initialization messages or errors

4. Check the Hone LSP server logs:
   ```
   ~/.local/state/hone/lsp.log  (Linux)
   ~/Library/Application Support/hone/lsp.log  (macOS)
   %LOCALAPPDATA%\hone\lsp.log  (Windows)
   ```

### Diagnostics not showing

Helix shows diagnostics automatically. If you don't see them:

1. Ensure there are actual errors in your `.hone` file
2. Check `:log-open` for LSP communication issues
3. Try closing and reopening the file

### Completion not working

1. Make sure you're triggering completion with `Ctrl+x` in insert mode
2. Check that the language server started successfully (`:log-open`)
3. Try restarting Helix

### Syntax highlighting issues

If syntax highlighting looks incorrect:

1. Helix may need a restart after adding language configuration
2. Verify the `file-types` setting includes `"hone"` in your `languages.toml`
3. The LSP provides semantic tokens for enhanced highlighting on supported versions

## Testing the Setup

Create a test file `test.hone`:

```hone
@test "example test" {
  run echo "Hello, World!"
  expect stdout {
    Hello
  }
  expect exitcode 0
}
```

Open it in Helix:
```sh
hx test.hone
```

You should see:
- Syntax highlighting
- No diagnostic errors
- Completion suggestions when typing keywords
- Hover documentation when pressing `Space+k` on keywords

## Additional Resources

- [Hone Documentation](https://github.com/captainsafia/hone)
- [Helix Editor Documentation](https://docs.helix-editor.com/)
- [Helix Language Configuration](https://docs.helix-editor.com/languages.html)
