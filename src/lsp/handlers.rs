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

pub fn handle_did_save(_state: &mut ServerState, params: DidSaveTextDocumentParams) {
    let uri = params.text_document.uri;
    tracing::info!("Document saved: {}", uri);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_uri(path: &str) -> Url {
        Url::parse(&format!("file://{}", path)).unwrap()
    }

    #[test]
    fn test_server_state_default() {
        let state = ServerState::default();
        assert!(state.documents.is_empty());
        assert!(!state.shutdown_requested);
        assert!(state.client.is_none());
    }

    #[test]
    fn test_server_state_new() {
        let state = ServerState::new();
        assert!(state.documents.is_empty());
        assert!(!state.shutdown_requested);
    }

    #[test]
    fn test_open_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls".to_string();

        state.open_document(uri.clone(), text.clone());

        assert_eq!(state.documents.len(), 1);
        assert_eq!(state.get_document(&uri), Some(&text));
    }

    #[test]
    fn test_update_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text1 = "TEST \"example\"\nRUN ls".to_string();
        let text2 = "TEST \"updated\"\nRUN pwd".to_string();

        state.open_document(uri.clone(), text1);
        state.update_document(&uri, text2.clone());

        assert_eq!(state.get_document(&uri), Some(&text2));
    }

    #[test]
    fn test_update_nonexistent_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"".to_string();

        state.update_document(&uri, text);

        assert_eq!(state.documents.len(), 0);
    }

    #[test]
    fn test_close_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"".to_string();

        state.open_document(uri.clone(), text);
        assert_eq!(state.documents.len(), 1);

        state.close_document(&uri);
        assert_eq!(state.documents.len(), 0);
        assert_eq!(state.get_document(&uri), None);
    }

    #[test]
    fn test_handle_initialize() {
        let params = InitializeParams::default();
        let result = handle_initialize(params);

        assert_eq!(result.server_info.unwrap().name, "hone-lsp");
        assert!(result.capabilities.text_document_sync.is_some());
        assert!(result.capabilities.completion_provider.is_some());
        assert!(result.capabilities.hover_provider.is_some());
        assert!(result.capabilities.document_symbol_provider.is_some());
        assert!(result.capabilities.document_formatting_provider.is_some());
        assert!(result.capabilities.semantic_tokens_provider.is_some());
    }

    #[test]
    fn test_handle_initialize_trigger_characters() {
        let params = InitializeParams::default();
        let result = handle_initialize(params);

        let completion_options = result.capabilities.completion_provider.unwrap();
        let triggers = completion_options.trigger_characters.unwrap();
        assert!(triggers.contains(&"@".to_string()));
        assert!(triggers.contains(&" ".to_string()));
    }

    #[test]
    fn test_handle_shutdown() {
        let mut state = ServerState::new();
        assert!(!state.shutdown_requested);

        handle_shutdown(&mut state);
        assert!(state.shutdown_requested);
    }

    #[test]
    fn test_handle_exit_after_shutdown() {
        let mut state = ServerState::new();
        state.shutdown_requested = true;

        let exit_code = handle_exit(&state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_handle_exit_without_shutdown() {
        let state = ServerState::new();

        let exit_code = handle_exit(&state);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_handle_did_open() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls".to_string();

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "hone".to_string(),
                version: 1,
                text: text.clone(),
            },
        };

        handle_did_open(&mut state, params);

        assert_eq!(state.get_document(&uri), Some(&text));
    }

    #[test]
    fn test_handle_did_change() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text1 = "TEST \"example\"\nRUN ls".to_string();
        let text2 = "TEST \"updated\"\nRUN pwd".to_string();

        state.open_document(uri.clone(), text1);

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text2.clone(),
            }],
        };

        handle_did_change(&mut state, params);

        assert_eq!(state.get_document(&uri), Some(&text2));
    }

    #[test]
    fn test_handle_did_change_empty_changes() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"".to_string();

        state.open_document(uri.clone(), text.clone());

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![],
        };

        handle_did_change(&mut state, params);

        assert_eq!(state.get_document(&uri), Some(&text));
    }

    #[test]
    fn test_handle_did_close() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"".to_string();

        state.open_document(uri.clone(), text);

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };

        handle_did_close(&mut state, params);

        assert_eq!(state.get_document(&uri), None);
    }

    #[test]
    fn test_handle_did_save() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"".to_string();

        state.open_document(uri.clone(), text.clone());

        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            text: Some(text.clone()),
        };

        handle_did_save(&mut state, params);

        // State should still have the document (didSave doesn't remove it)
        assert_eq!(state.get_document(&uri), Some(&text));
    }

    #[test]
    fn test_handle_completion_valid_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT exit_code == 0".to_string();

        state.open_document(uri.clone(), text);

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 1,
                    character: 4, // After "RUN "
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let result = handle_completion(&state, params);
        // Completion may or may not return results depending on context
        // Just verify the call doesn't crash
        let _ = result;
    }

    #[test]
    fn test_handle_completion_nonexistent_document() {
        let state = ServerState::new();
        let uri = create_test_uri("/nonexistent.hone");

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let result = handle_completion(&state, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_completion_invalid_syntax() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "INVALID SYNTAX HERE{{{".to_string();

        state.open_document(uri.clone(), text);

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 5,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let result = handle_completion(&state, params);
        // Parser is fault-tolerant, may still provide completions
        // Just verify the call doesn't crash
        let _ = result;
    }

    #[test]
    fn test_handle_hover_valid_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls".to_string();

        state.open_document(uri.clone(), text);

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 1, // Position on "EST" part of TEST
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handle_hover(&state, params);
        // Hover should work when positioned on a keyword
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_hover_nonexistent_document() {
        let state = ServerState::new();
        let uri = create_test_uri("/nonexistent.hone");

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handle_hover(&state, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_document_symbols_valid_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT exit_code == 0".to_string();

        state.open_document(uri.clone(), text);

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handle_document_symbols(&state, params);
        assert!(result.is_some());

        if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
            assert!(!symbols.is_empty());
        }
    }

    #[test]
    fn test_handle_document_symbols_nonexistent_document() {
        let state = ServerState::new();
        let uri = create_test_uri("/nonexistent.hone");

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handle_document_symbols(&state, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_document_symbols_invalid_syntax() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "INVALID {{{{".to_string();

        state.open_document(uri.clone(), text);

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handle_document_symbols(&state, params);
        // Parser is fault-tolerant, may still parse and return symbols
        // Just verify the call doesn't crash
        let _ = result;
    }

    #[test]
    fn test_handle_formatting_valid_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\n  RUN ls\n    ASSERT exit_code == 0".to_string();

        state.open_document(uri.clone(), text);

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options: FormattingOptions {
                tab_size: 2,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handle_formatting(&state, params);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_formatting_nonexistent_document() {
        let state = ServerState::new();
        let uri = create_test_uri("/nonexistent.hone");

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options: FormattingOptions {
                tab_size: 2,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handle_formatting(&state, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_semantic_tokens_valid_document() {
        let mut state = ServerState::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls".to_string();

        state.open_document(uri.clone(), text);

        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handle_semantic_tokens(&state, params);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_semantic_tokens_nonexistent_document() {
        let state = ServerState::new();
        let uri = create_test_uri("/nonexistent.hone");

        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handle_semantic_tokens(&state, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_documents() {
        let mut state = ServerState::new();
        let uri1 = create_test_uri("/test1.hone");
        let uri2 = create_test_uri("/test2.hone");
        let text1 = "TEST \"test1\"".to_string();
        let text2 = "TEST \"test2\"".to_string();

        state.open_document(uri1.clone(), text1.clone());
        state.open_document(uri2.clone(), text2.clone());

        assert_eq!(state.documents.len(), 2);
        assert_eq!(state.get_document(&uri1), Some(&text1));
        assert_eq!(state.get_document(&uri2), Some(&text2));

        state.close_document(&uri1);
        assert_eq!(state.documents.len(), 1);
        assert_eq!(state.get_document(&uri2), Some(&text2));
    }

    #[test]
    fn test_lifecycle_clean_exit() {
        let mut state = ServerState::new();

        handle_shutdown(&mut state);
        assert!(state.shutdown_requested);

        let exit_code = handle_exit(&state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_lifecycle_dirty_exit() {
        let state = ServerState::new();
        assert!(!state.shutdown_requested);

        let exit_code = handle_exit(&state);
        assert_eq!(exit_code, 1);
    }
}
