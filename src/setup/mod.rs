use anyhow::Result;

mod detect;
mod editors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editor {
    VSCode,
    Neovim,
    Vim,
}

impl Editor {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "vscode" | "code" => Some(Self::VSCode),
            "neovim" | "nvim" => Some(Self::Neovim),
            "vim" => Some(Self::Vim),
            _ => None,
        }
    }

    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::VSCode => "vscode",
            Self::Neovim => "neovim",
            Self::Vim => "vim",
        }
    }

    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::VSCode => &["code"],
            Self::Neovim => &["nvim"],
            Self::Vim => &[],
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::VSCode => "Visual Studio Code",
            Self::Neovim => "Neovim",
            Self::Vim => "Vim",
        }
    }

    pub fn all() -> &'static [Editor] {
        &[Self::VSCode, Self::Neovim, Self::Vim]
    }
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
    check_hone_in_path();

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

fn check_hone_in_path() {
    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(_) => return,
    };

    let exe_dir = match current_exe.parent() {
        Some(dir) => dir,
        None => return,
    };

    if let Ok(path_var) = std::env::var("PATH") {
        for path in std::env::split_paths(&path_var) {
            if path == exe_dir {
                return;
            }
        }
    }

    eprintln!(
        "Warning: 'hone' binary is not in PATH. Add {} to PATH for editors to find it.",
        exe_dir.display()
    );
}

fn setup_editor(editor: Editor) -> Result<()> {
    match editor {
        Editor::Neovim => editors::neovim::configure(),
        Editor::VSCode => editors::vscode::setup(),
        Editor::Vim => editors::vim::configure(),
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
    }

    #[test]
    fn test_editor_from_name_aliases() {
        assert_eq!(Editor::from_name("code"), Some(Editor::VSCode));
        assert_eq!(Editor::from_name("nvim"), Some(Editor::Neovim));
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
}
