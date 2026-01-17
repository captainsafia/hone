use async_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InsertTextFormat,
    Position,
};

use crate::parser::ast::ParsedFile;

#[derive(Debug, Clone)]
pub struct CompletionProvider;

impl CompletionProvider {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn provide_completions(
        &self,
        parsed: &ParsedFile,
        params: &CompletionParams,
    ) -> Option<CompletionResponse> {
        let position = params.text_document_position.position;
        let context = self.determine_context(parsed, position);

        let items = match context {
            CompletionContext::TopLevel => self.top_level_completions(),
            CompletionContext::InsideTest => self.inside_test_completions(),
            CompletionContext::AfterExpect => self.assertion_completions(),
            CompletionContext::Unknown => Vec::new(),
        };

        if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        }
    }

    fn determine_context(&self, _parsed: &ParsedFile, _position: Position) -> CompletionContext {
        // TODO: Implement proper context detection by walking the AST
        // For now, return a default context
        CompletionContext::TopLevel
    }

    fn top_level_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "@test".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Define a test block".to_string()),
                insert_text: Some("@test \"$1\" {\n\t$2\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "@setup".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Define a setup block".to_string()),
                insert_text: Some("@setup {\n\t$1\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    fn inside_test_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "expect".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Add an assertion".to_string()),
                insert_text: Some("expect ".to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            },
            CompletionItem {
                label: "run".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Execute a shell command".to_string()),
                insert_text: Some("run $1".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    fn assertion_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "stdout".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on standard output".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the standard output of the command".to_string(),
                )),
                insert_text: Some("stdout {\n\t$1\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "stderr".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on standard error".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the standard error of the command".to_string(),
                )),
                insert_text: Some("stderr {\n\t$1\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "exitcode".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on exit code".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the exit code of the command (0-255)".to_string(),
                )),
                insert_text: Some("exitcode $1".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "file".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on file contents".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the contents of a file".to_string(),
                )),
                insert_text: Some("file \"$1\" {\n\t$2\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "duration".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on execution duration".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check how long the command took to execute".to_string(),
                )),
                insert_text: Some("duration < $1".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionContext {
    TopLevel,
    InsideTest,
    AfterExpect,
    Unknown,
}
