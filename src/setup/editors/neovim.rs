use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::setup::detect::is_in_path;

pub fn is_installed() -> bool {
    is_in_path("nvim")
}

pub fn configure() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("Neovim not installed");
    }

    let config_dir = get_config_dir()?;
    fs::create_dir_all(&config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let (config_file, config_type) = detect_config_file(&config_dir)?;

    let lsp_client = prompt_lsp_client()?;

    let config_content = match config_type {
        ConfigType::Lua => generate_lua_config(&lsp_client),
        ConfigType::Vim => generate_vim_config(&lsp_client),
    };

    append_config(&config_file, &config_content, &config_type)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConfigType {
    Lua,
    Vim,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LspClient {
    NvimLspconfig,
    Manual,
    CocNvim,
}

fn get_config_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;

    #[cfg(target_os = "windows")]
    let config_dir = PathBuf::from(&home)
        .join("AppData")
        .join("Local")
        .join("nvim");

    #[cfg(not(target_os = "windows"))]
    let config_dir = PathBuf::from(&home).join(".config").join("nvim");

    Ok(config_dir)
}

fn detect_config_file(config_dir: &Path) -> Result<(PathBuf, ConfigType)> {
    let init_lua = config_dir.join("init.lua");
    let init_vim = config_dir.join("init.vim");

    if init_lua.exists() {
        Ok((init_lua, ConfigType::Lua))
    } else if init_vim.exists() {
        Ok((init_vim, ConfigType::Vim))
    } else {
        Ok((init_lua, ConfigType::Lua))
    }
}

fn prompt_lsp_client() -> Result<LspClient> {
    println!("\nSelect LSP client for Neovim:");
    println!("  1. nvim-lspconfig (recommended for nvim-lspconfig users)");
    println!("  2. Native LSP (manual configuration)");
    println!("  3. coc.nvim");
    print!("Enter choice [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => Ok(LspClient::NvimLspconfig),
        "2" => Ok(LspClient::Manual),
        "3" => Ok(LspClient::CocNvim),
        _ => {
            println!("Invalid choice, using nvim-lspconfig");
            Ok(LspClient::NvimLspconfig)
        }
    }
}

fn generate_lua_config(lsp_client: &LspClient) -> String {
    match lsp_client {
        LspClient::NvimLspconfig => r#"
-- Hone LSP configuration (nvim-lspconfig)
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.hone then
  configs.hone = {
    default_config = {
      cmd = { 'hone', 'lsp' },
      filetypes = { 'hone' },
      root_dir = lspconfig.util.root_pattern('.git', '.'),
      settings = {},
    },
  }
end

lspconfig.hone.setup {}

-- Associate .hone files with hone filetype
vim.filetype.add({
  extension = {
    hone = 'hone',
  },
})
"#
        .to_string(),
        LspClient::Manual => r#"
-- Hone LSP configuration (manual)
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'hone',
  callback = function()
    vim.lsp.start({
      name = 'hone',
      cmd = { 'hone', 'lsp' },
      root_dir = vim.fs.dirname(vim.fs.find({ '.git' }, { upward = true })[1]),
    })
  end,
})

-- Associate .hone files with hone filetype
vim.filetype.add({
  extension = {
    hone = 'hone',
  },
})
"#
        .to_string(),
        LspClient::CocNvim => r#"
" Hone LSP configuration (coc.nvim)
" Add this to your coc-settings.json:
" {
"   "languageserver": {
"     "hone": {
"       "command": "hone",
"       "args": ["lsp"],
"       "filetypes": ["hone"],
"       "rootPatterns": [".git/"]
"     }
"   }
" }

" Associate .hone files with hone filetype
au BufRead,BufNewFile *.hone set filetype=hone
"#
        .to_string(),
    }
}

fn generate_vim_config(lsp_client: &LspClient) -> String {
    match lsp_client {
        LspClient::CocNvim => r#"
" Hone LSP configuration (coc.nvim)
" Add this to your coc-settings.json:
" {
"   "languageserver": {
"     "hone": {
"       "command": "hone",
"       "args": ["lsp"],
"       "filetypes": ["hone"],
"       "rootPatterns": [".git/"]
"     }
"   }
" }

" Associate .hone files with hone filetype
au BufRead,BufNewFile *.hone set filetype=hone
"#
        .to_string(),
        _ => r#"
" Hone LSP configuration
" Native LSP in Neovim requires Lua configuration
" Please switch to init.lua or use coc.nvim for Vimscript configuration

" Associate .hone files with hone filetype
au BufRead,BufNewFile *.hone set filetype=hone
"#
        .to_string(),
    }
}

fn append_config(config_file: &Path, content: &str, config_type: &ConfigType) -> Result<()> {
    let marker_start = match config_type {
        ConfigType::Lua => "-- BEGIN HONE CONFIG",
        ConfigType::Vim => "\" BEGIN HONE CONFIG",
    };
    let marker_end = match config_type {
        ConfigType::Lua => "-- END HONE CONFIG",
        ConfigType::Vim => "\" END HONE CONFIG",
    };

    let existing_content = if config_file.exists() {
        fs::read_to_string(config_file)
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?
    } else {
        String::new()
    };

    if existing_content.contains(marker_start) {
        return Ok(());
    }

    let new_content = format!(
        "{}\n{}\n{}\n{}\n",
        existing_content.trim_end(),
        marker_start,
        content.trim(),
        marker_end
    );

    fs::write(config_file, new_content)
        .with_context(|| format!("Failed to write config file: {}", config_file.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lua_config_lspconfig() {
        let config = generate_lua_config(&LspClient::NvimLspconfig);
        assert!(config.contains("require('lspconfig')"));
        assert!(config.contains("cmd = { 'hone', 'lsp' }"));
        assert!(config.contains("filetypes = { 'hone' }"));
        assert!(config.contains("vim.filetype.add"));
    }

    #[test]
    fn test_generate_lua_config_manual() {
        let config = generate_lua_config(&LspClient::Manual);
        assert!(config.contains("vim.lsp.start"));
        assert!(config.contains("cmd = { 'hone', 'lsp' }"));
        assert!(config.contains("vim.filetype.add"));
    }

    #[test]
    fn test_generate_lua_config_coc() {
        let config = generate_lua_config(&LspClient::CocNvim);
        assert!(config.contains("coc-settings.json"));
        assert!(config.contains("au BufRead,BufNewFile *.hone"));
    }

    #[test]
    fn test_generate_vim_config_coc() {
        let config = generate_vim_config(&LspClient::CocNvim);
        assert!(config.contains("coc-settings.json"));
        assert!(config.contains("au BufRead,BufNewFile *.hone"));
    }

    #[test]
    fn test_generate_vim_config_native() {
        let config = generate_vim_config(&LspClient::Manual);
        assert!(config.contains("requires Lua configuration"));
        assert!(config.contains("au BufRead,BufNewFile *.hone"));
    }
}
