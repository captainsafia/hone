use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::setup::detect::is_in_path;

pub fn is_installed() -> bool {
    is_in_path("emacs")
}

pub fn configure() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("Emacs not installed");
    }

    let init_file = get_init_file()?;

    if let Some(parent) = init_file.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create emacs config directory: {}",
                parent.display()
            )
        })?;
    }

    let lsp_client = prompt_lsp_client()?;
    let config_content = generate_elisp_config(&lsp_client);

    append_config(&init_file, config_content)?;

    println!("Configured Emacs");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LspClient {
    Eglot,
    LspMode,
}

fn get_init_file() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let home_path = PathBuf::from(&home);

    let emacs_d_init = home_path.join(".emacs.d").join("init.el");
    if emacs_d_init.exists() {
        return Ok(emacs_d_init);
    }

    let dot_emacs = home_path.join(".emacs");
    if dot_emacs.exists() {
        return Ok(dot_emacs);
    }

    Ok(emacs_d_init)
}

fn prompt_lsp_client() -> Result<LspClient> {
    println!("\nChoose Emacs LSP client:");
    println!("  1) eglot (built-in, Emacs 29+)");
    println!("  2) lsp-mode");
    print!("Enter choice [1-2]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" | "eglot" => Ok(LspClient::Eglot),
        "2" | "lsp-mode" | "lsp" => Ok(LspClient::LspMode),
        _ => Ok(LspClient::Eglot),
    }
}

fn generate_elisp_config(lsp_client: &LspClient) -> &'static str {
    match lsp_client {
        LspClient::Eglot => generate_eglot_config(),
        LspClient::LspMode => generate_lsp_mode_config(),
    }
}

fn generate_eglot_config() -> &'static str {
    r#"
;; Hone LSP configuration (eglot)
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '(hone-mode . ("hone" "lsp"))))

;; Associate .hone files with hone-mode
(add-to-list 'auto-mode-alist '("\\.hone\\'" . hone-mode))

;; Define hone-mode if not already defined
(unless (fboundp 'hone-mode)
  (define-derived-mode hone-mode prog-mode "Hone"
    "Major mode for editing Hone test files."))

;; Auto-start eglot for hone files
(add-hook 'hone-mode-hook 'eglot-ensure)
"#
}

fn generate_lsp_mode_config() -> &'static str {
    r#"
;; Hone LSP configuration (lsp-mode)
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(hone-mode . "hone"))
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection '("hone" "lsp"))
                    :major-modes '(hone-mode)
                    :server-id 'hone-lsp)))

;; Associate .hone files with hone-mode
(add-to-list 'auto-mode-alist '("\\.hone\\'" . hone-mode))

;; Define hone-mode if not already defined
(unless (fboundp 'hone-mode)
  (define-derived-mode hone-mode prog-mode "Hone"
    "Major mode for editing Hone test files."))

;; Auto-start lsp for hone files
(add-hook 'hone-mode-hook 'lsp)
"#
}

fn append_config(init_file: &Path, config: &str) -> Result<()> {
    let marker_start = ";; BEGIN HONE LSP CONFIG";
    let marker_end = ";; END HONE LSP CONFIG";

    let existing_content = if init_file.exists() {
        fs::read_to_string(init_file)?
    } else {
        String::new()
    };

    if existing_content.contains(marker_start) {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(init_file)
        .with_context(|| format!("Failed to open init file: {}", init_file.display()))?;

    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        writeln!(file)?;
    }

    writeln!(file, "{}", marker_start)?;
    write!(file, "{}", config)?;
    writeln!(file, "{}", marker_end)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_eglot_config() {
        let config = generate_eglot_config();
        assert!(config.contains("eglot"));
        assert!(config.contains("hone-mode"));
        assert!(config.contains("(\"hone\" \"lsp\")"));
        assert!(config.contains("eglot-ensure"));
    }

    #[test]
    fn test_generate_lsp_mode_config() {
        let config = generate_lsp_mode_config();
        assert!(config.contains("lsp-mode"));
        assert!(config.contains("hone-mode"));
        assert!(config.contains("'(\"hone\" \"lsp\")"));
        assert!(config.contains("lsp-register-client"));
    }

    #[test]
    fn test_prompt_lsp_client_parsing() {
        assert_eq!(
            match "1" {
                "1" | "eglot" => LspClient::Eglot,
                "2" | "lsp-mode" | "lsp" => LspClient::LspMode,
                _ => LspClient::Eglot,
            },
            LspClient::Eglot
        );

        assert_eq!(
            match "2" {
                "1" | "eglot" => LspClient::Eglot,
                "2" | "lsp-mode" | "lsp" => LspClient::LspMode,
                _ => LspClient::Eglot,
            },
            LspClient::LspMode
        );
    }
}
