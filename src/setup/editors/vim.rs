use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::setup::detect::is_in_path;

pub fn is_installed() -> bool {
    if !is_in_path("vim") {
        return false;
    }

    let output = Command::new("vim").arg("--version").output();
    if let Ok(output) = output {
        let version = String::from_utf8_lossy(&output.stdout);
        !version.contains("NVIM")
    } else {
        false
    }
}

pub fn configure() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("Vim not installed");
    }

    let vimrc_path = get_vimrc_path()?;

    if let Some(parent) = vimrc_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create vim config directory: {}",
                parent.display()
            )
        })?;
    }

    append_vim_config(&vimrc_path)?;

    Ok(())
}

fn get_vimrc_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let home_path = PathBuf::from(&home);

    let vimrc = home_path.join(".vimrc");
    if vimrc.exists() {
        return Ok(vimrc);
    }

    let vim_vimrc = home_path.join(".vim").join("vimrc");
    if vim_vimrc.exists() {
        return Ok(vim_vimrc);
    }

    Ok(vimrc)
}

fn generate_vim_config() -> &'static str {
    r#"
" Hone LSP configuration (vim-lsp)
" Requires vim-lsp plugin: https://github.com/prabirshrestha/vim-lsp

if executable('hone')
  augroup LspHone
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
      \ 'name': 'hone',
      \ 'cmd': {server_info->['hone', 'lsp']},
      \ 'allowlist': ['hone'],
      \ })
  augroup END
endif

" Associate .hone files with hone filetype
au BufRead,BufNewFile *.hone set filetype=hone
"#
}

fn append_vim_config(vimrc_path: &Path) -> Result<()> {
    let marker_start = "\" BEGIN HONE CONFIG";
    let marker_end = "\" END HONE CONFIG";

    let existing_content = if vimrc_path.exists() {
        fs::read_to_string(vimrc_path)
            .with_context(|| format!("Failed to read vimrc: {}", vimrc_path.display()))?
    } else {
        String::new()
    };

    if existing_content.contains(marker_start) {
        return Ok(());
    }

    let config_content = generate_vim_config();
    let new_content = format!(
        "{}\n{}\n{}\n{}\n",
        existing_content.trim_end(),
        marker_start,
        config_content.trim(),
        marker_end
    );

    fs::write(vimrc_path, new_content)
        .with_context(|| format!("Failed to write vimrc: {}", vimrc_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_vim_config() {
        let config = generate_vim_config();
        assert!(config.contains("vim-lsp"));
        assert!(config.contains("hone"));
        assert!(config.contains("lsp#register_server"));
        assert!(config.contains("['hone', 'lsp']"));
        assert!(config.contains("au BufRead,BufNewFile *.hone set filetype=hone"));
    }

    #[test]
    fn test_vim_config_is_idempotent() {
        let config = generate_vim_config();
        let marker_start = "\" BEGIN HONE CONFIG";
        let marker_end = "\" END HONE CONFIG";

        let content1 = format!("{}\n{}\n{}\n", marker_start, config.trim(), marker_end);

        let combined = format!("{}{}", content1, content1);
        assert!(combined.contains(marker_start));
        assert_eq!(combined.matches(marker_start).count(), 2);
    }
}
