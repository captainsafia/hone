# Specification: `hone setup` Command

## Overview

The `hone setup` command automatically installs and configures the Hone LSP and syntax highlighting for supported code editors.

## Command Interface

```
hone setup [EDITOR...]
```

### Arguments

- `EDITOR...` - One or more editor names to configure (optional)

### Behavior

- **No arguments**: Lists all supported editors and usage information
- **One or more editors**: Configures each specified editor

### Supported Editors and Aliases

| Canonical Name | Aliases            | Notes                                     |
|----------------|--------------------|--------------------------------------------|
| `vscode`       | `code`             | Visual Studio Code                        |
| `neovim`       | `nvim`             | Neovim                                    |
| `vim`          |                    | Vim (not Neovim)                          |

### Examples

```sh
# List available editors
hone setup

# Configure single editor
hone setup vscode

# Configure multiple editors
hone setup vscode neovim

# Using aliases
hone setup code nvim
```

## Configuration Details

### Scope

All configurations are applied at the **user level** (global), not workspace/project level.

### What Gets Configured

For each editor, the command configures:
1. **LSP client** - Points to `hone lsp` subcommand
2. **Syntax highlighting** - TextMate grammar or editor-native highlighting
3. **File associations** - Associates `.hone` extension with the hone filetype

### Binary Path Resolution

- The configuration uses `hone` (expecting it to be in PATH)
- If the currently executing binary is not in a PATH directory, emit a warning to stderr

## Editor-Specific Behavior

### VS Code (`vscode`, `code`)

**Detection**: Check for VS Code installation via:
- macOS: `/Applications/Visual Studio Code.app` or `code` in PATH
- Linux: `code` in PATH or `/usr/share/code`
- Windows: Registry or `%LOCALAPPDATA%\Programs\Microsoft VS Code`

**Configuration files**:
- Settings: `~/.config/Code/User/settings.json` (Linux), `~/Library/Application Support/Code/User/settings.json` (macOS), `%APPDATA%\Code\User\settings.json` (Windows)

**Configured elements** (all via settings.json, no extension required):
- LSP configuration pointing to `hone lsp`
- File association for `.hone` files
- TextMate grammar for syntax highlighting (embedded in settings)

**Note**: VS Code support is implemented purely through user settings configuration. No separate VS Code extension is published or installed.

### Neovim (`neovim`, `nvim`)

**Detection**: Check for `nvim` in PATH

**Configuration approach**: Detect config format and adapt:
- If `init.lua` exists: Generate Lua configuration
- If `init.vim` exists: Generate Vimscript configuration
- If neither exists: Create `init.lua` with Lua configuration

**LSP client options** (prompt user to choose):
1. Native LSP (nvim-lspconfig) - for nvim-lspconfig users
2. Native LSP (manual) - for users configuring LSP manually
3. coc.nvim - for coc.nvim users

**Configuration locations**:
- Linux/macOS: `~/.config/nvim/`
- Windows: `~/AppData/Local/nvim/`

### Vim (`vim`)

**Detection**: Check for `vim` in PATH (and verify it's not a symlink to nvim)

**LSP client**: vim-lsp (prabirshrestha/vim-lsp)

**Configuration files**:
- `~/.vimrc` or `~/.vim/vimrc`

**Note**: Requires user to have vim-lsp plugin installed; command adds configuration for it.

## Conflict Handling

When existing configuration is detected that would conflict:

1. **Prompt user interactively** with options:
   - Overwrite existing configuration
   - Skip this editor
   - Abort entire operation

2. For structured config files (JSON, TOML):
   - Detect if hone-related configuration already exists
   - Only prompt if there's actual conflict

## Error Handling

### Editor Not Installed

If the specified editor is not detected on the system:
- **Error**: Exit with error message indicating editor was not found
- **Exit code**: Non-zero
- Does not attempt to configure

### Multi-Editor Errors

When configuring multiple editors:
- **Continue on error**: Attempt all editors regardless of individual failures
- **Report at end**: Summary showing which editors succeeded/failed
- **Exit code**: Non-zero if any editor failed

### Config File Creation

If configuration files don't exist:
- **Create them**: Automatically create necessary files and directories
- Parent directories created as needed with appropriate permissions

## Output

### Default (Minimal)

```
Configured VS Code
Configured Neovim
```

On error:
```
Configured VS Code
Error: Neovim not installed
```

### List Mode (no arguments)

```
Available editors:
  vscode (code)      - Visual Studio Code
  neovim (nvim)      - Neovim
  vim                - Vim

Usage: hone setup <editor> [<editor>...]

Example: hone setup vscode neovim
```

## Exit Codes

| Code | Meaning                                    |
|------|--------------------------------------------|
| 0    | All specified editors configured successfully |
| 1    | One or more editors failed to configure    |
| 2    | Invalid arguments or usage error           |

## Non-Goals

- No VS Code extension (all VS Code support via settings.json configuration)
- No uninstall/removal command (document manual removal in help text)
- No `--dry-run` flag
- No customization via config files or environment variables
- No workspace-level configuration
- No verification step after configuration
- No special handling if hone binary moves after setup

## Implementation Notes

### File Modification Strategy

For JSON files (VS Code):
- Parse existing JSON
- Merge hone configuration
- Preserve formatting where possible (use serde_json with pretty printing)

For Vimscript/Lua:
- Append configuration to end of file
- Use clear comment markers to identify hone-added sections

### Platform Detection

Use `std::env::consts::OS` for OS detection:
- `linux`
- `macos`
- `windows`

Config paths should use appropriate separators and expand `~` correctly.

### Dependencies

No external dependencies required beyond standard library and existing crate dependencies.

## Implementation Checklist

### Phase 1: Foundation

- [ ] Create `src/setup/` module directory
- [ ] Create `src/setup/mod.rs` with module structure
- [ ] Add `setup` subcommand to CLI parser in `src/main.rs`
- [ ] Implement editor enum with canonical names and aliases
- [ ] Implement alias resolution (e.g., `nvim` → `neovim`)
- [ ] Add platform detection utilities (OS-specific config paths)
- [ ] Implement home directory expansion (`~` handling)

### Phase 2: Core Infrastructure

- [ ] Create `src/setup/detect.rs` for editor detection
- [ ] Implement PATH lookup utility
- [ ] Implement binary existence check for each editor
- [ ] Create `src/setup/config.rs` for config file operations
- [ ] Implement JSON config read/merge/write (for VS Code)
- [ ] Implement append-with-markers for text configs (Vim, Neovim)
- [ ] Implement conflict detection for existing hone configuration
- [ ] Implement interactive prompt for conflict resolution

### Phase 3: Editor Implementations

#### VS Code
- [ ] Create `src/setup/editors/vscode.rs`
- [ ] Implement VS Code detection (PATH + app locations per OS)
- [ ] Implement settings.json path resolution per OS
- [ ] Generate LSP configuration JSON
- [ ] Generate file association configuration
- [ ] Embed TextMate grammar in settings

#### Neovim
- [ ] Create `src/setup/editors/neovim.rs`
- [ ] Implement Neovim detection
- [ ] Detect init.lua vs init.vim
- [ ] Implement LSP client selection prompt
- [ ] Generate nvim-lspconfig Lua snippet
- [ ] Generate manual LSP Lua snippet
- [ ] Generate coc.nvim JSON configuration
- [ ] Generate Vimscript equivalent for init.vim users

#### Vim
- [ ] Create `src/setup/editors/vim.rs`
- [ ] Implement Vim detection (excluding nvim symlinks)
- [ ] Detect vimrc location
- [ ] Generate vim-lsp configuration snippet

### Phase 4: Command Integration

- [ ] Implement `hone setup` (no args) - list editors
- [ ] Implement single editor setup flow
- [ ] Implement multi-editor setup with continue-on-error
- [ ] Implement final success/failure summary
- [ ] Implement exit codes (0, 1, 2)
- [ ] Add PATH warning for hone binary
- [ ] Wire up interactive prompts for conflicts and LSP client selection

### Phase 5: Testing

- [ ] Unit tests for alias resolution
- [ ] Unit tests for platform path detection
- [ ] Unit tests for JSON merge logic
- [ ] Unit tests for TOML merge logic
- [ ] Unit tests for conflict detection
- [ ] Integration test: `hone setup` lists editors
- [ ] Integration test: `hone setup` with invalid editor name
- [ ] Integration test: setup with editor not installed
- [ ] Manual testing on Linux
- [ ] Manual testing on macOS
- [ ] Manual testing on Windows (if available)

### Phase 6: Documentation

- [ ] Add `hone setup --help` documentation
- [ ] Document manual removal steps in help output
- [ ] Update README with setup command usage
- [ ] Add troubleshooting section for common issues

## Success Criteria

### Functional Requirements

- [ ] `hone setup` with no arguments displays list of supported editors
- [ ] `hone setup <editor>` configures the specified editor
- [ ] `hone setup <editor1> <editor2>` configures multiple editors in sequence
- [ ] Editor aliases work (e.g., `nvim` for `neovim`, `code` for `vscode`)
- [ ] Command fails with clear error if editor is not installed
- [ ] Interactive prompt appears when existing config conflicts are detected
- [ ] Missing config files/directories are created automatically
- [ ] Warning is emitted if `hone` binary is not in PATH

### Editor-Specific Success Criteria

#### VS Code
- [ ] After setup, opening a `.hone` file activates the LSP
- [ ] Syntax highlighting is applied to `.hone` files
- [ ] Go-to-definition, hover, and diagnostics work

#### Neovim
- [ ] Setup detects init.lua vs init.vim and generates appropriate config
- [ ] User is prompted to choose LSP client (nvim-lspconfig, manual, coc.nvim)
- [ ] After setup, LSP attaches when opening `.hone` files

#### Vim
- [ ] vim-lsp configuration is added to vimrc
- [ ] After setup (with vim-lsp installed), LSP features work

### Cross-Platform Requirements

- [ ] Works on Linux (tested on Ubuntu/Debian)
- [ ] Works on macOS (tested on Apple Silicon and Intel)
- [ ] Works on Windows (tested on Windows 10/11)
- [ ] Config paths are correct for each OS

### Error Handling

- [ ] When one editor fails in multi-editor setup, others still proceed
- [ ] Final summary shows which editors succeeded/failed
- [ ] Exit code is non-zero if any editor failed
- [ ] Clear error messages for common failure modes

### User Experience

- [ ] Setup completes in under 5 seconds for any single editor
- [ ] No unnecessary prompts when there are no conflicts
- [ ] Output is concise and actionable
