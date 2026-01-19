use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const INSTALLER_BASE_URL: &str = "https://i.safia.sh/captainsafia/hone";

pub async fn perform_update(version: Option<String>) -> Result<()> {
    let url = match &version {
        Some(v) => format!("{}/{}", INSTALLER_BASE_URL, v),
        None => INSTALLER_BASE_URL.to_string(),
    };

    match &version {
        Some(v) => println!("Downloading installer script for v{}...", v),
        None => println!("Downloading installer script..."),
    }

    let response = reqwest::get(&url)
        .await
        .context("Failed to download installer script")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download installer script: HTTP {}",
            response.status()
        );
    }

    let script_content = response
        .text()
        .await
        .context("Failed to read installer script")?;

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let script_path = temp_dir.path().join("install.sh");
    std::fs::write(&script_path, &script_content).context("Failed to write installer script")?;

    let mut child = Command::new("sh")
        .arg(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to execute installer script")?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_handle = std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", line);
            }
        }
    });

    let stderr_handle = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("{}", line);
            }
        }
    });

    stdout_handle.join().ok();
    stderr_handle.join().ok();

    let status = child.wait().context("Failed to wait for installer")?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
