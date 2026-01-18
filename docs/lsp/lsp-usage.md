# Hone Language Server Usage Guide

This document provides detailed setup instructions and troubleshooting tips for using Hone's Language Server Protocol (LSP) implementation.

## Overview

Hone's LSP provides IDE features for `.hone` test files, including:

- **Diagnostics**: Real-time syntax, semantic, and type error reporting
- **Completion**: Context-aware keyword, assertion, and shell command suggestions with snippets
- **Hover**: Documentation for keywords and assertions
- **Document Symbols**: Outline view of test structure
- **Formatting**: Automatic code formatting with consistent indentation
- **Semantic Tokens**: Enhanced syntax highlighting

## Starting the Language Server

```sh
hone lsp
```

The server:
- Communicates via stdio (stdin/stdout)
- Follows the [Language Server Protocol specification](https://microsoft.github.io/language-server-protocol/)
- Requires no configuration (zero-config design)
- Works with any LSP-compatible editor

## Editor Setup Guides

### Visual Studio Code

#### Option 1: Install the Extension (Recommended)

1. Navigate to the extension directory:
   ```sh
   cd editors/vscode
   ```

2. Install dependencies and compile:
   ```sh
   npm install
   npm run compile
   ```

3. Install the extension:
   ```sh
   code --install-extension .
   ```

4. Reload VS Code

#### Option 2: Manual Configuration

Add to your `settings.json`:

```json
{
  "hone.languageServer.enabled": true,
  "hone.languageServer.command": "hone",
  "hone.languageServer.args": ["lsp"],
  "files.associations": {
    "*.hone": "hone"
  }
}
```

### Neovim

#### Using nvim-lspconfig

1. Install `nvim-lspconfig` if not already installed:
   ```lua
   -- Using lazy.nvim
   { 'neovim/nvim-lspconfig' }
   ```

2. Configure the Hone LSP in your `init.lua`:
   ```lua
   local lspconfig = require('lspconfig')
   local configs = require('lspconfig.configs')

   -- Define hone LSP configuration
   if not configs.hone then
     configs.hone = {
       default_config = {
         cmd = { 'hone', 'lsp' },
         filetypes = { 'hone' },
         root_dir = lspconfig.util.root_pattern('.git', '.hone'),
         single_file_support = true,
         settings = {},
       },
       docs = {
         description = [[
           Language server for Hone integration test files.
           Provides diagnostics, completion, hover, and formatting.
         ]],
       },
     }
   end

   -- Setup the server
   lspconfig.hone.setup{
     on_attach = function(client, bufnr)
       -- Optional: Add keybindings here
       local opts = { buffer = bufnr, noremap = true, silent = true }
       vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
       vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
       vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
       vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
     end,
     capabilities = require('cmp_nvim_lsp').default_capabilities(),
   }
   ```

3. Set filetype detection. Create `~/.config/nvim/ftdetect/hone.vim`:
   ```vim
   au BufRead,BufNewFile *.hone set filetype=hone
   ```

   Or add to your `init.lua`:
   ```lua
   vim.filetype.add({
     extension = {
       hone = 'hone',
     },
   })
   ```

#### Using Manual LSP Setup

If not using `nvim-lspconfig`, you can start the client manually:

```lua
vim.lsp.start({
  name = 'hone',
  cmd = { 'hone', 'lsp' },
  root_dir = vim.fs.dirname(vim.fs.find({'.git', '.hone'}, { upward = true })[1]),
})
```

### Helix

1. Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "hone"
scope = "source.hone"
file-types = ["hone"]
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
language-servers = ["hone-lsp"]

[language-server.hone-lsp]
command = "hone"
args = ["lsp"]
```

2. Restart Helix

3. Open a `.hone` file to verify the LSP is working

### Zed

1. Add to your Zed configuration (`~/.config/zed/settings.json`):

```json
{
  "languages": {
    "Hone": {
      "language_servers": ["hone-lsp"],
      "file_types": ["hone"]
    }
  },
  "lsp": {
    "hone-lsp": {
      "binary": {
        "path": "hone",
        "arguments": ["lsp"]
      }
    }
  }
}
```

### Sublime Text

Using LSP package:

1. Install the [LSP package](https://github.com/sublimelsp/LSP)

2. Add to LSP settings (`Preferences → Package Settings → LSP → Settings`):

```json
{
  "clients": {
    "hone": {
      "enabled": true,
      "command": ["hone", "lsp"],
      "selector": "source.hone",
      "file_patterns": ["*.hone"]
    }
  }
}
```

3. Create syntax definition for `.hone` files or use TextMate grammar from `syntaxes/hone.tmlanguage.json`

### Emacs

Using `lsp-mode`:

1. Add to your Emacs configuration:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(hone-mode . "hone"))
  
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("hone" "lsp"))
    :activation-fn (lsp-activate-on "hone")
    :server-id 'hone-lsp)))

(define-derived-mode hone-mode prog-mode "Hone"
  "Major mode for editing Hone integration test files."
  (setq-local comment-start "# ")
  (setq-local comment-end ""))

(add-to-list 'auto-mode-alist '("\\.hone\\'" . hone-mode))
```

## Features in Detail

### Diagnostics

The LSP reports three types of errors:

1. **Syntax Errors**: Invalid `.hone` syntax
   - Example: Missing closing brace, invalid characters
   - Appears as red squiggles in most editors

2. **Semantic Errors**: Invalid test structure
   - Example: `expect` outside of `@test` block
   - Example: `env` inside `@test` block (must be at top level)

3. **Type Errors**: Incorrect assertion argument types
   - Example: `exitcode "foo"` (should be a number)
   - Error messages include helpful suggestions

### Completion

Completions appear automatically as you type. Trigger completion manually with your editor's completion shortcut (usually `Ctrl+Space`).

**Keywords**:
- `@test` - Creates a new test block with snippet
- `@setup` - Creates a setup block
- `expect` - Assertion keyword (inside tests)
- `run` - Execute command (inside tests)
- `env` - Set environment variable (top-level)

**Assertions** (after typing `expect `):
- `stdout` - Assert on standard output
- `stderr` - Assert on standard error
- `exitcode` - Assert on exit code
- `file` - Assert on file contents/existence
- `duration` - Assert on command duration

**Shell Commands**:
- Common commands: `ls`, `cd`, `cat`, `echo`, `grep`, `git`, `npm`, `cargo`, etc.
- Scans your PATH for available executables

**Snippets** include tab stops for quick navigation:
```
@test "$1" {
  $2
}
```

Press Tab to move between `$1` (test name) and `$2` (test body).

### Hover

Hover over keywords and assertions to see documentation:
- Keyword documentation explains syntax and usage
- Assertion documentation includes examples
- Works with mouse hover or keyboard shortcut (usually `K` in Vim/Neovim)

### Document Symbols (Outline)

The outline view shows:
- All `@test` blocks with their names
- All `@setup` blocks
- Nested `expect` assertions within tests

Access via:
- VS Code: `Ctrl+Shift+O` or the Outline panel
- Neovim: `:Telescope lsp_document_symbols`
- Helix: `Space+s` (symbol picker)

### Formatting

Format your document with consistent style:
- Top-level blocks at column 0
- Block contents indented (2 spaces by default)
- Normalized spacing around braces

Access via:
- VS Code: `Shift+Alt+F`
- Neovim: `vim.lsp.buf.format()`
- Helix: `:format`

Shell command internal formatting is preserved.

### Semantic Highlighting

Enhanced syntax highlighting with token types:
- Keywords: `@test`, `@setup`, `expect`
- Test names: strings
- Assertion names: functions
- Shell commands: macros
- Comments: dimmed

Requires editor support for semantic tokens (VS Code, Neovim 0.9+, Helix).

## Logging and Debugging

### Log Location

LSP logs are written to platform-specific locations:

- **Linux**: `~/.local/state/hone/lsp.log`
- **macOS**: `~/Library/Application Support/hone/lsp.log`
- **Windows**: `%LOCALAPPDATA%\hone\lsp.log`

### Viewing Logs

```sh
# Linux/macOS
tail -f ~/.local/state/hone/lsp.log

# Follow logs in real-time
tail -f "$(hone lsp --log-path)"  # If this flag exists
```

### Log Contents

Logs include:
- Server startup and shutdown events
- Request/response messages
- Parse errors and diagnostics
- Internal errors and warnings

### Increasing Log Verbosity

By default, logs are at INFO level. The LSP currently doesn't expose configuration for log levels, but you can check the log file for errors if features aren't working.

## Troubleshooting

### LSP Not Starting

**Problem**: Editor shows "LSP server failed to start"

**Solutions**:
1. Verify `hone` is in your PATH:
   ```sh
   which hone
   hone --version
   ```

2. Test the LSP manually:
   ```sh
   hone lsp
   ```
   It should wait for input (press Ctrl+C to exit)

3. Check editor LSP client logs:
   - VS Code: Output panel → "Hone Language Server"
   - Neovim: `:LspLog`
   - Helix: Check console output

### No Diagnostics Appearing

**Problem**: Syntax errors don't show up as squiggles

**Solutions**:
1. Verify the file is recognized as `.hone`:
   - Check the file extension
   - Verify filetype detection in your editor

2. Check if diagnostics are enabled in your editor settings

3. Review LSP logs for parse errors

### Completion Not Working

**Problem**: No completions appear when typing

**Solutions**:
1. Ensure completion is enabled in your editor
2. Try manual completion trigger (usually `Ctrl+Space`)
3. Check if you're in a valid completion context:
   - Keywords work at top-level or inside blocks
   - Assertions work after `expect `
   - Shell commands work after `run `

### Hover Not Showing Documentation

**Problem**: Hovering shows nothing

**Solutions**:
1. Verify hover is enabled in editor LSP settings
2. Ensure you're hovering over a keyword or assertion name
3. Check if your editor supports LSP hover (most do)

### Formatting Does Nothing

**Problem**: Format command doesn't change the file

**Solutions**:
1. Verify the file is syntactically valid (check diagnostics)
2. Ensure formatting is enabled in editor settings
3. Check if another formatter is overriding the LSP formatter

### Semantic Highlighting Not Working

**Problem**: No enhanced colors, just basic syntax highlighting

**Solutions**:
1. Check if your editor supports semantic tokens:
   - VS Code: Yes (built-in)
   - Neovim: Requires version 0.9+
   - Helix: Yes (built-in)
   - Older editors: May not support semantic tokens

2. Verify TextMate grammar is loaded as fallback:
   - Check `syntaxes/hone.tmlanguage.json` exists
   - Install grammar in editor if needed

3. Semantic tokens are an enhancement; basic highlighting should still work

## Performance Considerations

### Workspace Size

The LSP uses lazy loading:
- Only parses files when opened
- No eager indexing on startup
- Minimal memory footprint

Large workspaces (1000+ `.hone` files) should perform well since only active files are analyzed.

### Parse Performance

Typical parse times:
- Small file (10 tests): <1ms
- Large file (100 tests): <10ms

If you experience slowness, check the LSP log for errors.

## Version Compatibility

### Forward Compatibility

The LSP gracefully handles newer `.hone` syntax:
- Unknown blocks are skipped
- Rest of the file continues to be analyzed
- Optional warning for unrecognized syntax

This allows older LSP versions to work with newer test files.

### Backward Compatibility

Newer LSP versions support all historical `.hone` syntax. No breaking changes to the DSL are expected.

## Contributing

### Reporting Issues

If you encounter LSP issues:

1. Check this guide's troubleshooting section
2. Review the LSP log file
3. Open an issue with:
   - Editor and version
   - Hone version (`hone --version`)
   - Log snippets showing the error
   - Minimal `.hone` file reproducing the issue

### Feature Requests

The LSP follows a zero-configuration philosophy. Feature requests should:
- Work across all editors
- Require no configuration
- Follow LSP specification conventions

## Further Reading

- [Hone Main README](../README.md) - General Hone documentation
- [LSP Specification](./LSP_SPEC.md) - Technical implementation details
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) - Official LSP spec
- [Hone DSL Specification](./SPEC.md) - Complete `.hone` syntax reference
