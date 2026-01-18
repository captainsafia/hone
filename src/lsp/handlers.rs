use async_lsp::lsp_types::*;
use async_lsp::{ClientSocket, LanguageClient};
use std::collections::HashMap;

use crate::lsp::completion::CompletionProvider;
use crate::lsp::formatting::FormattingProvider;
use crate::lsp::hover::HoverProvider;
use crate::lsp::semantic_tokens::SemanticTokensProvider;
use crate::lsp::symbols::SymbolsProvider;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub documents: HashMap<Url, String>,
    pub shutdown_requested: bool,
    pub client: Option<ClientSocket>,
    pub completion_provider: CompletionProvider,
    pub hover_provider: HoverProvider,
    pub symbols_provider: SymbolsProvider,
    pub formatting_provider: FormattingProvider,
    pub semantic_tokens_provider: SemanticTokensProvider,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            documents: HashMap::new(),
            shutdown_requested: false,
            client: None,
            completion_provider: CompletionProvider::new(),
            hover_provider: HoverProvider::new(),
            symbols_provider: SymbolsProvider::new(),
            formatting_provider: FormattingProvider::new(),
            semantic_tokens_provider: SemanticTokensProvider::new(),
        }
    }
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

    let semantic_tokens_provider = SemanticTokensProvider::new();
    let legend = semantic_tokens_provider.legend();

    InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec!["@".to_string(), " ".to_string()]),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    legend,
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                }),
            ),
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

pub fn handle_completion(
    state: &ServerState,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    tracing::debug!("Completion requested for: {}", uri);

    let text = state.get_document(uri)?;

    // Parse the document to get the AST
    let filename = uri.path();
    let parsed = match crate::parser::parse_file(text, filename) {
        crate::parser::ParseResult::Success { file } => file,
        crate::parser::ParseResult::Failure { errors, .. } => {
            tracing::warn!("Failed to parse document for completion: {:?}", errors);
            return None;
        }
    };

    state
        .completion_provider
        .provide_completions(&parsed, &params, text)
}

pub fn handle_hover(state: &ServerState, params: HoverParams) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    tracing::debug!("Hover requested for: {}", uri);

    let text = state.get_document(uri)?;

    state.hover_provider.provide_hover(text, &params)
}

pub fn handle_document_symbols(
    state: &ServerState,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    tracing::debug!("Document symbols requested for: {}", uri);

    let text = state.get_document(uri)?;

    // Parse the document to get the AST
    let filename = uri.path();
    let parsed = match crate::parser::parse_file(text, filename) {
        crate::parser::ParseResult::Success { file } => file,
        crate::parser::ParseResult::Failure { errors, .. } => {
            tracing::warn!("Failed to parse document for symbols: {:?}", errors);
            return None;
        }
    };

    let symbols = state.symbols_provider.provide_symbols(&parsed);
    Some(DocumentSymbolResponse::Nested(symbols))
}

pub fn handle_formatting(
    state: &ServerState,
    params: DocumentFormattingParams,
) -> Option<Vec<TextEdit>> {
    let uri = &params.text_document.uri;
    tracing::debug!("Formatting requested for: {}", uri);

    let text = state.get_document(uri)?;

    state.formatting_provider.format_document(text, uri.path())
}

pub fn handle_semantic_tokens(
    state: &ServerState,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = &params.text_document.uri;
    tracing::debug!("Semantic tokens requested for: {}", uri);

    let text = state.get_document(uri)?;

    state
        .semantic_tokens_provider
        .provide_semantic_tokens(uri, text)
}
