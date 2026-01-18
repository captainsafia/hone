use std::path::Path;
use std::process::Command;

pub fn is_in_path(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn detect_vscode() -> bool {
    if is_in_path("code") {
        return true;
    }

    if cfg!(target_os = "macos") {
        Path::new("/Applications/Visual Studio Code.app").exists()
    } else if cfg!(target_os = "windows") {
        Path::new(&format!(
            "{}\\Microsoft VS Code",
            std::env::var("LOCALAPPDATA").unwrap_or_default()
        ))
        .exists()
    } else {
        Path::new("/usr/share/code").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_path() {
        assert!(is_in_path("sh"));
        assert!(!is_in_path("nonexistent_command_xyz"));
    }
}
