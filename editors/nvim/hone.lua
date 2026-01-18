-- Neovim LSP configuration for Hone
-- This file provides a complete setup for the Hone language server.
--
-- Installation:
-- 1. Ensure you have nvim-lspconfig installed
-- 2. Add this configuration to your init.lua or init.vim
-- 3. Make sure the `hone` binary is in your PATH
--
-- Basic setup (add to your init.lua):
--
--   require('lspconfig.configs').hone = {
--     default_config = {
--       cmd = { 'hone', 'lsp' },
--       filetypes = { 'hone' },
--       root_dir = function(fname)
--         return require('lspconfig.util').find_git_ancestor(fname) or vim.fn.getcwd()
--       end,
--       settings = {},
--     },
--   }
--   require('lspconfig').hone.setup{}
--
-- Advanced setup with custom on_attach:
--
--   local on_attach = function(client, bufnr)
--     local opts = { noremap=true, silent=true, buffer=bufnr }
--     vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
--     vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
--     vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
--     vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
--     vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
--     vim.keymap.set('n', '<leader>f', vim.lsp.buf.format, opts)
--   end
--
--   require('lspconfig.configs').hone = {
--     default_config = {
--       cmd = { 'hone', 'lsp' },
--       filetypes = { 'hone' },
--       root_dir = function(fname)
--         return require('lspconfig.util').find_git_ancestor(fname) or vim.fn.getcwd()
--       end,
--       settings = {},
--     },
--   }
--   require('lspconfig').hone.setup{
--     on_attach = on_attach,
--   }

local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')
local util = require('lspconfig.util')

-- Register the Hone language server
if not configs.hone then
  configs.hone = {
    default_config = {
      cmd = { 'hone', 'lsp' },
      filetypes = { 'hone' },
      root_dir = function(fname)
        return util.find_git_ancestor(fname) or vim.fn.getcwd()
      end,
      single_file_support = true,
      settings = {},
      init_options = {},
    },
    docs = {
      description = [[
https://github.com/captainsafia/hone

Language server for Hone, a CLI tool for integration testing of command-line applications.

The language server provides:
- Syntax and semantic diagnostics
- Code completion for keywords, assertions, and shell commands
- Hover documentation
- Document outline
- Formatting
- Semantic syntax highlighting
]],
    },
  }
end

-- Setup the server with default configuration
lspconfig.hone.setup{}

return configs.hone
