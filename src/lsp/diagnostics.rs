use crate::parser::{parse_file, ParseResult};
use async_lsp::lsp_types::*;

pub fn generate_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
    let path = uri.path();
    let parse_result = parse_file(content, path);

    let mut diagnostics = Vec::new();

    match parse_result {
        ParseResult::Success { file } => {
            // Process errors from error nodes in the AST
            for error in &file.errors {
                diagnostics.push(create_diagnostic_from_parse_error(error));
            }

            // Process warnings
            for warning in &file.warnings {
                diagnostics.push(create_diagnostic_from_warning(warning));
            }

            // Process Error nodes in the AST
            for node in &file.nodes {
                if let crate::parser::ASTNode::Error(error_node) = node {
                    diagnostics.push(create_diagnostic_from_error_node(error_node));
                }
            }
        }
        ParseResult::Failure { errors, warnings } => {
            // If parsing completely failed, report all errors
            for error in &errors {
                diagnostics.push(create_diagnostic_from_parse_error(error));
            }

            for warning in &warnings {
                diagnostics.push(create_diagnostic_from_warning(warning));
            }
        }
    }

    diagnostics
}

fn create_diagnostic_from_parse_error(error: &crate::parser::ParseErrorDetail) -> Diagnostic {
    // Line numbers in the parser are 1-indexed, LSP uses 0-indexed
    let line = if error.line > 0 { error.line - 1 } else { 0 };

    Diagnostic {
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: u32::MAX, // Highlight entire line
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("hone".to_string()),
        message: error.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn create_diagnostic_from_warning(warning: &crate::parser::ParseWarning) -> Diagnostic {
    let line = if warning.line > 0 {
        warning.line - 1
    } else {
        0
    };

    Diagnostic {
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: u32::MAX,
            },
        },
        severity: Some(DiagnosticSeverity::WARNING),
        code: None,
        code_description: None,
        source: Some("hone".to_string()),
        message: warning.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn create_diagnostic_from_error_node(error_node: &crate::parser::ErrorNode) -> Diagnostic {
    let span = &error_node.span;

    // Parser uses 1-indexed lines, LSP uses 0-indexed
    let start_line = if span.start_line > 0 {
        span.start_line - 1
    } else {
        0
    };
    let end_line = if span.end_line > 0 {
        span.end_line - 1
    } else {
        0
    };

    Diagnostic {
        range: Range {
            start: Position {
                line: start_line as u32,
                character: span.start_col as u32,
            },
            end: Position {
                line: end_line as u32,
                character: span.end_col as u32,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("hone".to_string()),
        message: error_node.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}
