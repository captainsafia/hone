# Hone LSP for Neovim

This directory contains the Neovim configuration for the Hone language server.

## Prerequisites

- Neovim 0.8.0 or later
- [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) plugin
- The `hone` binary in your PATH

## Quick Setup

Add the following to your `init.lua`:

```lua
-- Configure the Hone language server
require('lspconfig.configs').hone = {
  default_config = {
    cmd = { 'hone', 'lsp' },
    filetypes = { 'hone' },
    root_dir = function(fname)
      return require('lspconfig.util').find_git_ancestor(fname) or vim.fn.getcwd()
    end,
    settings = {},
  },
}

-- Start the server
require('lspconfig').hone.setup{}
```

## Filetype Detection

Neovim needs to recognize `.hone` files. Add this to your config:

```lua
-- Add to init.lua
vim.filetype.add({
  extension = {
    hone = 'hone',
  },
})
```

Or if using `init.vim`:

```vim
" Add to init.vim
autocmd BufNewFile,BufRead *.hone set filetype=hone
```

## Full Configuration Example

For a complete setup with keybindings:

```lua
-- Filetype detection
vim.filetype.add({
  extension = {
    hone = 'hone',
  },
})

-- Configure Hone LSP
require('lspconfig.configs').hone = {
  default_config = {
    cmd = { 'hone', 'lsp' },
    filetypes = { 'hone' },
    root_dir = function(fname)
      return require('lspconfig.util').find_git_ancestor(fname) or vim.fn.getcwd()
    end,
    settings = {},
  },
}

-- Custom on_attach function for keybindings
local on_attach = function(client, bufnr)
  local opts = { noremap = true, silent = true, buffer = bufnr }
  
  -- Hover documentation
  vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
  
  -- Format document
  vim.keymap.set('n', '<leader>f', vim.lsp.buf.format, opts)
  
  -- Show diagnostics
  vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float, opts)
  vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, opts)
  vim.keymap.set('n', ']d', vim.diagnostic.goto_next, opts)
  
  -- Document symbols (outline)
  vim.keymap.set('n', '<leader>o', vim.lsp.buf.document_symbol, opts)
end

-- Start the server with custom on_attach
require('lspconfig').hone.setup{
  on_attach = on_attach,
}
```

## Features

The Hone language server provides:

- **Diagnostics**: Real-time syntax, semantic, and type checking errors
- **Completion**: Context-aware completions for:
  - Keywords (`@test`, `@setup`, `expect`, `run`)
  - Assertions (`stdout`, `stderr`, `exitcode`, `file`, `duration`)
  - Shell commands (common commands + executables in PATH)
  - Code snippets with tab stops
- **Hover**: Documentation for keywords and assertions
- **Document Symbols**: Outline view showing test structure
- **Formatting**: Normalize indentation and spacing
- **Semantic Highlighting**: Enhanced syntax highlighting (requires Neovim 0.9+)

## Completion

The LSP provides rich completion with snippets:

- Type `@test` and accept the completion to insert a complete test block
- Type `expect ` (with trailing space) to see available assertions
- Use `<Tab>` to jump between snippet placeholders

## Troubleshooting

### Server not starting

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

3. Check LSP logs:
   ```
   ~/.local/state/hone/lsp.log  (Linux/macOS)
   ```

4. Enable verbose LSP logging in Neovim:
   ```lua
   vim.lsp.set_log_level("debug")
   ```
   Then check: `:lua print(vim.lsp.get_log_path())`

### Diagnostics not showing

Make sure diagnostics are enabled:
```lua
vim.diagnostic.config({
  virtual_text = true,
  signs = true,
  update_in_insert = false,
})
```

### Completion not working

Ensure you have a completion plugin configured:
- [nvim-cmp](https://github.com/hrsh7th/nvim-cmp) (recommended)
- [coq_nvim](https://github.com/ms-jpq/coq_nvim)
- Built-in omnifunc (`:set omnifunc=v:lua.vim.lsp.omnifunc`)

## Integration with nvim-cmp

If using nvim-cmp for completion:

```lua
local cmp = require('cmp')
cmp.setup({
  sources = {
    { name = 'nvim_lsp' },
    -- other sources...
  },
  snippet = {
    expand = function(args)
      -- You need a snippet engine like luasnip or vsnip
      require('luasnip').lsp_expand(args.body)
    end,
  },
})
```

## Additional Resources

- [Hone Documentation](https://github.com/captainsafia/hone)
- [nvim-lspconfig Documentation](https://github.com/neovim/nvim-lspconfig)
- [Neovim LSP Guide](https://neovim.io/doc/user/lsp.html)
