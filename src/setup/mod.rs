use anyhow::Result;
use std::path::PathBuf;

mod detect;
mod editors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editor {
    VSCode,
    Neovim,
    Vim,
    Helix,
    Emacs,
    Sublime,
    Zed,
}

impl Editor {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "vscode" | "code" => Some(Self::VSCode),
            "neovim" | "nvim" => Some(Self::Neovim),
            "vim" => Some(Self::Vim),
            "helix" | "hx" => Some(Self::Helix),
            "emacs" => Some(Self::Emacs),
            "sublime" | "subl" | "sublimetext" => Some(Self::Sublime),
            "zed" => Some(Self::Zed),
            _ => None,
        }
    }

    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::VSCode => "vscode",
            Self::Neovim => "neovim",
            Self::Vim => "vim",
            Self::Helix => "helix",
            Self::Emacs => "emacs",
            Self::Sublime => "sublime",
            Self::Zed => "zed",
        }
    }

    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::VSCode => &["code"],
            Self::Neovim => &["nvim"],
            Self::Vim => &[],
            Self::Helix => &["hx"],
            Self::Emacs => &[],
            Self::Sublime => &["subl", "sublimetext"],
            Self::Zed => &[],
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::VSCode => "Visual Studio Code",
            Self::Neovim => "Neovim",
            Self::Vim => "Vim",
            Self::Helix => "Helix",
            Self::Emacs => "GNU Emacs",
            Self::Sublime => "Sublime Text",
            Self::Zed => "Zed",
        }
    }

    pub fn all() -> &'static [Editor] {
        &[
            Self::VSCode,
            Self::Neovim,
            Self::Vim,
            Self::Helix,
            Self::Emacs,
            Self::Sublime,
            Self::Zed,
        ]
    }
}

fn expand_home(path: &str) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join(stripped))
    } else {
        Ok(PathBuf::from(path))
    }
}

pub fn get_config_path(path: &str) -> Result<PathBuf> {
    expand_home(path)
}

pub fn list_editors() {
    println!("Available editors:");
    for editor in Editor::all() {
        let aliases = editor.aliases();
        let alias_str = if aliases.is_empty() {
            String::new()
        } else {
            format!(" ({})", aliases.join(", "))
        };
        println!(
            "  {:<18} - {}",
            format!("{}{}", editor.canonical_name(), alias_str),
            editor.description()
        );
    }
    println!();
    println!("Usage: hone setup <editor> [<editor>...]");
    println!();
    println!("Example: hone setup vscode neovim");
}

pub fn setup_editors(editor_names: Vec<String>) -> Result<()> {
    let mut editors = Vec::new();

    for name in &editor_names {
        match Editor::from_name(name) {
            Some(editor) => editors.push(editor),
            None => {
                anyhow::bail!("Unknown editor: {}", name);
            }
        }
    }

    let mut had_error = false;

    for editor in editors {
        match setup_editor(editor) {
            Ok(()) => {
                println!("Configured {}", editor.description());
            }
            Err(e) => {
                eprintln!("Error: {}: {}", editor.description(), e);
                had_error = true;
            }
        }
    }

    if had_error {
        anyhow::bail!("One or more editors failed to configure");
    }

    Ok(())
}

fn setup_editor(editor: Editor) -> Result<()> {
    match editor {
        Editor::Helix => editors::helix::setup(),
        Editor::Neovim => editors::neovim::configure(),
        Editor::VSCode => editors::vscode::setup(),
        Editor::Vim => editors::vim::configure(),
        Editor::Emacs => editors::emacs::configure(),
        Editor::Zed => editors::zed::setup(),
        Editor::Sublime => editors::sublime::setup(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_from_name_canonical() {
        assert_eq!(Editor::from_name("vscode"), Some(Editor::VSCode));
        assert_eq!(Editor::from_name("neovim"), Some(Editor::Neovim));
        assert_eq!(Editor::from_name("vim"), Some(Editor::Vim));
        assert_eq!(Editor::from_name("helix"), Some(Editor::Helix));
        assert_eq!(Editor::from_name("emacs"), Some(Editor::Emacs));
        assert_eq!(Editor::from_name("sublime"), Some(Editor::Sublime));
        assert_eq!(Editor::from_name("zed"), Some(Editor::Zed));
    }

    #[test]
    fn test_editor_from_name_aliases() {
        assert_eq!(Editor::from_name("code"), Some(Editor::VSCode));
        assert_eq!(Editor::from_name("nvim"), Some(Editor::Neovim));
        assert_eq!(Editor::from_name("hx"), Some(Editor::Helix));
        assert_eq!(Editor::from_name("subl"), Some(Editor::Sublime));
        assert_eq!(Editor::from_name("sublimetext"), Some(Editor::Sublime));
    }

    #[test]
    fn test_editor_from_name_case_insensitive() {
        assert_eq!(Editor::from_name("VSCODE"), Some(Editor::VSCode));
        assert_eq!(Editor::from_name("Code"), Some(Editor::VSCode));
        assert_eq!(Editor::from_name("NeoVim"), Some(Editor::Neovim));
    }

    #[test]
    fn test_editor_from_name_invalid() {
        assert_eq!(Editor::from_name("invalid"), None);
        assert_eq!(Editor::from_name(""), None);
    }

    #[test]
    fn test_canonical_name() {
        assert_eq!(Editor::VSCode.canonical_name(), "vscode");
        assert_eq!(Editor::Neovim.canonical_name(), "neovim");
        assert_eq!(Editor::Vim.canonical_name(), "vim");
    }

    #[test]
    fn test_expand_home() {
        let result = expand_home("~/test/path");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("test/path"));
        assert!(!path.to_string_lossy().contains("~"));
    }

    #[test]
    fn test_expand_home_no_tilde() {
        let result = expand_home("/absolute/path");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/absolute/path"));
    }
}
