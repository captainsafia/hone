use async_lsp::lsp_types::*;
use async_lsp::{ClientSocket, LanguageClient};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ServerState {
    pub documents: HashMap<Url, String>,
    pub shutdown_requested: bool,
    pub client: Option<ClientSocket>,
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_document(&mut self, uri: Url, text: String) {
        tracing::debug!("Opening document: {}", uri);
        self.documents.insert(uri, text);
    }

    pub fn update_document(&mut self, uri: &Url, text: String) {
        tracing::debug!("Updating document: {}", uri);
        if let Some(content) = self.documents.get_mut(uri) {
            *content = text;
        } else {
            tracing::warn!("Attempted to update non-existent document: {}", uri);
        }
    }

    pub fn close_document(&mut self, uri: &Url) {
        tracing::debug!("Closing document: {}", uri);
        self.documents.remove(uri);
    }

    pub fn get_document(&self, uri: &Url) -> Option<&String> {
        self.documents.get(uri)
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

pub fn handle_did_open(state: &mut ServerState, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri.clone();
    let text = params.text_document.text.clone();

    tracing::info!("Document opened: {}", uri);
    state.open_document(uri.clone(), text.clone());

    // Generate and publish diagnostics
    if let Some(client) = &mut state.client {
        let diagnostics = crate::lsp::diagnostics::generate_diagnostics(&uri, &text);

        let diagnostic_params = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: Some(params.text_document.version),
        };

        // Send diagnostics notification
        if let Err(e) = client.publish_diagnostics(diagnostic_params) {
            tracing::error!("Failed to publish diagnostics: {:?}", e);
        }
    }
}

pub fn handle_did_change(state: &mut ServerState, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri.clone();
    let version = params.text_document.version;

    // We're using full document sync, so there should be exactly one change with the full text
    if let Some(change) = params.content_changes.into_iter().next() {
        let text = change.text.clone();
        tracing::info!("Document changed: {}", uri);
        state.update_document(&uri, text.clone());

        // Generate and publish diagnostics
        if let Some(client) = &mut state.client {
            let diagnostics = crate::lsp::diagnostics::generate_diagnostics(&uri, &text);

            let diagnostic_params = PublishDiagnosticsParams {
                uri: uri.clone(),
                diagnostics,
                version: Some(version),
            };

            // Send diagnostics notification
            if let Err(e) = client.publish_diagnostics(diagnostic_params) {
                tracing::error!("Failed to publish diagnostics: {:?}", e);
            }
        }
    } else {
        tracing::warn!("Received didChange with no content changes for: {}", uri);
    }
}

pub fn handle_did_close(state: &mut ServerState, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri;

    tracing::info!("Document closed: {}", uri);
    state.close_document(&uri);
}
