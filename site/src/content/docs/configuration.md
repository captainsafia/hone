---
title: Configuration
description: Configure Hone through pragmas, CLI, and editor integration
order: 5
---

Hone can be configured through file pragmas, CLI options, and editor integration. This page covers all configuration options.

## File Pragmas

Pragmas at the top of a `.hone` file configure that file's test execution:

```hone
#! shell: /bin/zsh
#! env: NODE_ENV=test
#! env: DEBUG=true
#! timeout: 30s

TEST "my test"
RUN echo $NODE_ENV
ASSERT stdout contains "test"
```

| Pragma | Description | Default |
|--------|-------------|---------|
| `shell:` | Shell executable path | `/bin/sh` |
| `env:` | Environment variable (can repeat) | — |
| `timeout:` | Max execution time per command | — |

## CLI Options

The `hone` CLI accepts several options:

```bash
# Use a custom shell
hone run --shell /bin/zsh tests/

# Show verbose output (useful for debugging)
hone run --verbose tests/

# Show version
hone --version

# Show help
hone --help
```

## Language Server (LSP)

Hone includes a built-in Language Server that provides IDE features for `.hone` files.

### Features

- **Diagnostics** — Real-time syntax and semantic checking
- **Completion** — Context-aware snippets for keywords and assertions
- **Hover** — Documentation for keywords and assertions
- **Outline** — Document structure view
- **Formatting** — Consistent indentation and spacing
- **Semantic Highlighting** — Rich syntax highlighting

### Starting the Server

```bash
# Start the language server
hone lsp
```

### Editor Setup

Use the `setup` command to automatically configure your editor:

```bash
# Configure VS Code
hone setup vscode

# Configure Neovim
hone setup neovim

# Configure multiple editors
hone setup vscode neovim

# List supported editors
hone setup
```

### Manual VS Code Configuration

Add to your `settings.json`:

```json
{
  "hone.languageServer": {
    "enabled": true,
    "command": "hone",
    "args": ["lsp"]
  }
}
```

### Manual Neovim Configuration

With `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.hone then
  configs.hone = {
    default_config = {
      cmd = { 'hone', 'lsp' },
      filetypes = { 'hone' },
      root_dir = lspconfig.util.root_pattern('.git'),
      single_file_support = true,
    },
  }
end

lspconfig.hone.setup{}
```

## Exit Codes

Hone uses the following exit codes:

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed |

## LSP Logs

LSP logs are written to platform-specific locations:

- **Linux** — `~/.local/state/hone/lsp.log`
- **macOS** — `~/Library/Application Support/hone/lsp.log`
- **Windows** — `%LOCALAPPDATA%\hone\lsp.log`

---

**You're all set!** Check out the [Examples](/examples) page to see Hone in action with real-world test files.
