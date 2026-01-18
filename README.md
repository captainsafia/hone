# Hone

Hone is a CLI integration test runner for command-line applications, inspired by [Hurl](https://hurl.dev).

Write tests in a simple, line-oriented DSL that executes real shell commands and asserts on output, exit codes, file contents, and timing.

## Features

- File assertions (exists, contains, matches, equals) and duration assertions
- Persistent shell sessions (`cd`, variables, and state carry across commands)
- Separate stdout/stderr capture
- ANSI escape code stripping (or assert on raw output with `stdout.raw`)
- Language Server Protocol (LSP) support for editor integration

## Language Server (LSP)

Hone includes a built-in Language Server that provides IDE features for `.hone` test files.

### Features

- **Diagnostics**: Real-time syntax, semantic, and type checking
- **Completion**: Context-aware snippets for keywords, assertions, and shell commands
- **Hover**: Documentation for keywords and assertions
- **Outline**: Document structure view showing tests and setup blocks
- **Formatting**: Consistent indentation and spacing
- **Semantic Highlighting**: Rich syntax highlighting

### Starting the Language Server

```sh
hone lsp
```

The server communicates over stdio and follows the LSP specification.

### Editor Setup

Hone provides a `setup` command to automatically configure your editor:

```sh
# Configure a single editor
hone setup vscode

# Configure multiple editors
hone setup vscode neovim

# List all supported editors
hone setup
```

Supported editors:
- **VS Code** (`vscode`, `code`) - Visual Studio Code
- **Neovim** (`neovim`, `nvim`) - Neovim
- **Vim** (`vim`) - Vim

The setup command automatically configures:
- LSP (Language Server Protocol) integration
- Syntax highlighting for `.hone` files
- File associations

To remove the configuration, see `hone setup --help` for manual removal instructions.

#### Manual Configuration

If you prefer to configure manually, here are examples for some editors:

##### VS Code

Install the Hone extension from the `editors/vscode` directory:

```sh
cd editors/vscode
npm install
npm run compile
code --install-extension .
```

Or configure manually in your `settings.json`:

```json
{
  "hone.languageServer": {
    "enabled": true,
    "command": "hone",
    "args": ["lsp"]
  }
}
```

##### Neovim

With `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define hone LSP
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

-- Setup hone LSP
lspconfig.hone.setup{}
```

Set the filetype for `.hone` files in `~/.config/nvim/ftdetect/hone.vim`:

```vim
au BufRead,BufNewFile *.hone set filetype=hone
```

##### Other Editors

Any editor with LSP support can use Hone's language server. Configure it to:
- Run `hone lsp` as the server command
- Associate `.hone` files with the language server
- Use stdio for communication

### Logging

LSP logs are written to:
- **Linux**: `~/.local/state/hone/lsp.log`
- **macOS**: `~/Library/Application Support/hone/lsp.log`
- **Windows**: `%LOCALAPPDATA%\hone\lsp.log`

Check logs if you encounter issues with the language server.

### Troubleshooting

#### LSP not working after setup

1. **Verify hone is in PATH**: Run `which hone` (or `where hone` on Windows) to ensure the binary is accessible
2. **Check editor configuration**: Use `hone setup <editor> --help` to see manual removal instructions and verify configuration
3. **Restart editor**: Some editors require a restart to pick up new LSP configurations
4. **Check LSP logs**: See the Logging section above for log file locations

#### "hone not installed" error

The editor binary was not detected. Ensure:
- The editor is installed and accessible from the command line
- For macOS apps, check if they're in `/Applications/`
- For Linux, ensure the editor binary is in your `PATH`

#### Path warning when running setup

If you see a warning about hone not being in PATH:
- Add the hone binary directory to your system's `PATH` environment variable
- Or use an absolute path in editor configurations (not recommended)

#### Configuration changes not taking effect

- **Neovim/Vim**: Reload your init file or restart the editor
- **VS Code**: Reload the window (Command/Ctrl+Shift+P → "Developer: Reload Window")

#### Still having issues?

- Check that you're running a recent version of hone: `hone --version`
- Verify the `.hone` file syntax is correct by running tests: `hone test.hone`
- Check editor-specific LSP troubleshooting documentation

## Installation

### Using the installer (recommended)

```sh
# Install latest release
curl https://i.safia.sh/captainsafia/hone | sh

# Install a specific version
curl https://i.safia.sh/captainsafia/hone/v1.0.0 | sh

# Install latest prerelease
curl https://i.safia.sh/captainsafia/hone/preview | sh
```

## Quick Start

Create a test file `example.hone`:

```
#! shell: /bin/bash

TEST "echo works"

RUN echo "Hello, World!"
ASSERT exit_code == 0
ASSERT stdout contains "Hello"

TEST "file creation"

RUN echo "test content" > output.txt
ASSERT file "output.txt" exists
ASSERT file "output.txt" contains "test content"

TEST "error handling"

RUN ls nonexistent
ASSERT exit_code != 0
ASSERT stderr contains "No such file"
```

Run the tests:

```sh
hone run example.hone
```

## DSL Reference

### Pragmas

File-level configuration at the top of the file:

```
#! shell: /bin/zsh
#! env: MY_VAR=value
#! timeout: 60s
```

### TEST Blocks

Group related commands and assertions:

```
TEST "my test name"
```

Tests share the same shell session but have isolated ENV variables.

### RUN Commands

Execute shell commands:

```
RUN mycli init
RUN build: mycli build --release
```

Named runs (like `build:`) can be referenced in assertions.

### ENV Statements

Set environment variables for a test:

```
ENV API_KEY=test123
ENV DEBUG=true
```

### Assertions

#### Output Assertions

```
ASSERT stdout contains "expected text"
ASSERT stdout == "exact match"
ASSERT stdout != "not this"
ASSERT stdout matches /pattern/i
ASSERT stderr contains "error"
ASSERT stdout.raw contains "\x1b[32m"
```

#### Exit Code Assertions

```
ASSERT exit_code == 0
ASSERT exit_code != 0
```

#### Timing Assertions

```
ASSERT duration < 500ms
ASSERT duration <= 2s
```

#### File Assertions

```
ASSERT file "path/to/file.txt" exists
ASSERT file "output.json" contains "success"
ASSERT file "config.yaml" matches /version: \d+/
ASSERT file "exact.txt" == "exact content"
```

#### Named Target Assertions

Reference a specific RUN command:

```
RUN build: make build
RUN test: make test
ASSERT build.exit_code == 0
ASSERT build.duration < 30s
ASSERT test.stdout contains "PASSED"
```

### Comments

```
# This is a comment
```

## CLI Usage

```sh
# Run a single test file
hone run tests/integration.hone

# Run multiple test files
hone run tests/cli.hone tests/api.hone

# Run all files in a directory (recursively)
hone run tests/

# Run with a glob pattern (shell expands the glob)
hone run tests/*.hone

# Use a custom shell
hone run --shell /bin/zsh tests/

# Show verbose output on failures
hone run --verbose tests/

# Show version
hone --version

# Show help
hone --help
```

## Example Test Files

See the [examples/](examples/) directory for more comprehensive examples:

- `basic.hone` — Simple command and file assertions
- `assertions.hone` — All assertion types
- `environment.hone` — Environment variable handling
- `filesystem.hone` — File system assertions
- `shell-state.hone` — Shell state persistence
- `strings.hone` — String literal and escape handling

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed |

## Tips

### Testing exit codes

To test a command that exits the shell (like `exit 42`), wrap it in a subshell:

```
RUN (exit 42)
ASSERT exit_code == 42
```

### Literal vs escaped strings

- Single quotes are literal: `'hello\n'` contains backslash-n
- Double quotes support escapes: `"hello\n"` contains a newline

### Shell state

Commands share shell state within a file:

```
RUN cd /tmp
RUN pwd
ASSERT stdout contains "/tmp"
```

## Development Setup

```sh
# Clone the repository
git clone https://github.com/captainsafia/hone.git
cd hone

# Build the project
cargo build

# Run tests
cargo test

# Run integration tests
cargo run -- tests/integration/*.hone

# Run all example tests
cargo run -- examples/*.hone

# Build release version
cargo build --release

# Install locally
cargo install --path .
```