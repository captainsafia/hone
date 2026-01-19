use anyhow::{Context, Result};
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::ClientSocket;
use std::ops::ControlFlow;
use std::path::PathBuf;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tower::ServiceBuilder;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::lsp::handlers::ServerState;

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
    use async_lsp::lsp_types::notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Exit, Initialized,
    };
    use async_lsp::lsp_types::request::{
        Completion, DocumentSymbolRequest, Formatting, HoverRequest, Initialize,
        SemanticTokensFullRequest, Shutdown,
    };

    // Initialize logging first
    init_logging().context("Failed to initialize logging")?;

    // Log version and environment information on startup
    tracing::info!("Starting Hone LSP server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Process ID: {}", std::process::id());
    tracing::info!("Current directory: {:?}", std::env::current_dir().ok());
    tracing::info!("Executable path: {:?}", std::env::current_exe().ok());

    // Create the main loop and setup router
    let (mainloop, _client) = async_lsp::MainLoop::new_server(|client: ClientSocket| {
        let mut state = ServerState::new();
        state.client = Some(client);

        let mut router = Router::new(state);
        router
            .request::<Initialize, _>(|_state, params| async move {
                tracing::info!("Handling initialize request");
                let result = crate::lsp::handlers::handle_initialize(params);
                Ok(result)
            })
            .notification::<Initialized>(|state, params| {
                tracing::info!("Client initialized");
                crate::lsp::handlers::handle_initialized(state, params);
                ControlFlow::Continue(())
            })
            .notification::<DidOpenTextDocument>(|state, params| {
                crate::lsp::handlers::handle_did_open(state, params);
                ControlFlow::Continue(())
            })
            .notification::<DidChangeTextDocument>(|state, params| {
                crate::lsp::handlers::handle_did_change(state, params);
                ControlFlow::Continue(())
            })
            .notification::<DidCloseTextDocument>(|state, params| {
                crate::lsp::handlers::handle_did_close(state, params);
                ControlFlow::Continue(())
            })
            .notification::<DidSaveTextDocument>(|state, params| {
                crate::lsp::handlers::handle_did_save(state, params);
                ControlFlow::Continue(())
            })
            .request::<Completion, _>(|state, params| {
                let state = state.clone();
                async move {
                    tracing::debug!("Handling completion request");
                    let result = crate::lsp::handlers::handle_completion(&state, params);
                    Ok(result)
                }
            })
            .request::<HoverRequest, _>(|state, params| {
                let state = state.clone();
                async move {
                    tracing::debug!("Handling hover request");
                    let result = crate::lsp::handlers::handle_hover(&state, params);
                    Ok(result)
                }
            })
            .request::<DocumentSymbolRequest, _>(|state, params| {
                let state = state.clone();
                async move {
                    tracing::debug!("Handling document symbols request");
                    let result = crate::lsp::handlers::handle_document_symbols(&state, params);
                    Ok(result)
                }
            })
            .request::<Formatting, _>(|state, params| {
                let state = state.clone();
                async move {
                    tracing::debug!("Handling formatting request");
                    let result = crate::lsp::handlers::handle_formatting(&state, params);
                    Ok(result)
                }
            })
            .request::<SemanticTokensFullRequest, _>(|state, params| {
                let state = state.clone();
                async move {
                    tracing::debug!("Handling semantic tokens request");
                    let result = crate::lsp::handlers::handle_semantic_tokens(&state, params);
                    Ok(result)
                }
            })
            .request::<Shutdown, _>(|state, _params| {
                tracing::info!("Shutdown requested");
                let mut state = state.clone();
                async move {
                    crate::lsp::handlers::handle_shutdown(&mut state);
                    Ok(())
                }
            })
            .notification::<Exit>(|state, _params| {
                tracing::info!("Exit notification received");
                let exit_code = crate::lsp::handlers::handle_exit(state);
                std::process::exit(exit_code);
            });

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .service(router)
    });

    // Set up stdio transport - convert tokio types to futures types
    let stdin = tokio::io::stdin().compat();
    let stdout = tokio::io::stdout().compat_write();

    tracing::info!("Starting LSP main loop");

    // Run the main loop
    mainloop.run_buffered(stdin, stdout).await?;

    tracing::info!("LSP server shutdown complete");

    Ok(())
}
