use std::process::Command;

pub fn is_in_path(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn detect_helix() -> bool {
    is_in_path("hx") || is_in_path("helix")
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
