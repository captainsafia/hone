use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Get the platform-specific log directory for Hone LSP
fn get_log_dir() -> Result<PathBuf> {
    let log_dir = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        // Unix-like systems: use XDG_STATE_HOME or ~/.local/state
        if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
            PathBuf::from(state_home).join("hone")
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("hone")
        } else {
            anyhow::bail!("Cannot determine home directory for log file");
        }
    } else if cfg!(target_os = "windows") {
        // Windows: use LocalAppData
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(local_app_data).join("hone")
        } else {
            anyhow::bail!("Cannot determine LocalAppData directory for log file");
        }
    } else {
        anyhow::bail!("Unsupported platform for log file location");
    };

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

    Ok(log_dir)
}

/// Initialize file logging for the LSP server
fn init_logging() -> Result<()> {
    let log_dir = get_log_dir()?;

    // Create a rolling file appender that rotates daily
    // This prevents unbounded log growth
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir.clone(), "lsp.log");

    // Set up the tracing subscriber with file output
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // No ANSI codes in log files
        .with_target(true)
        .with_line_number(true);

    // Use RUST_LOG environment variable for filtering, default to INFO
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .init();

    tracing::info!(
        "Hone LSP server logging initialized (log file: {})",
        log_dir.join("lsp.log").display()
    );

    Ok(())
}

pub async fn run_lsp_server() -> Result<()> {
    // Initialize logging first
    init_logging().context("Failed to initialize logging")?;

    // Log version information on startup
    tracing::info!("Starting Hone LSP server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Build information: {}", env!("CARGO_PKG_VERSION"));

    // TODO: Implement LSP server initialization and main loop
    // This will:
    // 1. Set up stdio transport
    // 2. Initialize LSP server with capabilities
    // 3. Handle incoming requests via handlers module
    // 4. Run until shutdown is requested

    tracing::info!("LSP server initialized successfully");

    Ok(())
}
