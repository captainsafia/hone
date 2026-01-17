use async_lsp::lsp_types::*;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct ServerState {
    pub documents: HashMap<Url, String>,
    pub shutdown_requested: bool,
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn handle_initialize(_params: InitializeParams) -> InitializeResult {
    tracing::info!("Handling initialize request");

    InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "hone-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    }
}

pub fn handle_initialized(_state: &mut ServerState, _params: InitializedParams) {
    tracing::info!("Client initialized");
}

pub fn handle_shutdown(state: &mut ServerState) {
    tracing::info!("Shutdown requested");
    state.shutdown_requested = true;
}

pub fn handle_exit(state: &ServerState) -> i32 {
    tracing::info!("Exit requested");
    if state.shutdown_requested {
        tracing::info!("Clean exit (shutdown was called)");
        0
    } else {
        tracing::warn!("Exit without shutdown - returning error code");
        1
    }
}
