use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::process::Command;

use crate::setup::detect::detect_vscode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn setup() -> Result<()> {
    if !detect_vscode() {
        anyhow::bail!("VS Code not installed");
    }

    let vsix_url = format!(
        "https://github.com/captainsafia/hone/releases/download/v{}/hone-{}.vsix",
        VERSION, VERSION
    );

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let vsix_path = temp_dir.path().join(format!("hone-{}.vsix", VERSION));

    let vsix_path_str = vsix_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("VSIX path contains invalid UTF-8"))?;

    let response = Command::new("curl")
        .args(["-fsSL", "-o", vsix_path_str, &vsix_url])
        .output()
        .context("Failed to download VSIX")?;

    if !response.status.success() {
        let stderr = String::from_utf8_lossy(&response.stderr);
        anyhow::bail!("Failed to download VSIX from {}: {}", vsix_url, stderr);
    }

    if !vsix_path.exists() {
        anyhow::bail!("VSIX file was not downloaded");
    }

    let file_size = fs::metadata(&vsix_path)
        .context("Failed to get VSIX file metadata")?
        .len();
    if file_size == 0 {
        anyhow::bail!("Downloaded VSIX file is empty");
    }

    let install_result = Command::new("code")
        .args(["--install-extension", vsix_path_str])
        .output()
        .context("Failed to run 'code --install-extension'")?;

    if !install_result.status.success() {
        let stderr = String::from_utf8_lossy(&install_result.stderr);
        anyhow::bail!("Failed to install VS Code extension: {}", stderr);
    }

    std::io::stderr().write_all(&install_result.stderr)?;

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_vsix_url_format() {
        let version = "1.0.0";
        let url = format!(
            "https://github.com/captainsafia/hone/releases/download/v{}/hone-{}.vsix",
            version, version
        );
        assert_eq!(
            url,
            "https://github.com/captainsafia/hone/releases/download/v1.0.0/hone-1.0.0.vsix"
        );
    }
}
