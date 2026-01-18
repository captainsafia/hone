use crate::parser::{
    parse_file, ASTNode, AssertionExpression, ComparisonOperator, FilePredicate, OutputPredicate,
    ParseResult,
};
use async_lsp::lsp_types::*;

pub fn generate_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
    let path = uri.path();
    let parse_result = parse_file(content, path);

    let mut diagnostics = Vec::new();

    match parse_result {
        ParseResult::Success { file } => {
            // Process warnings
            for warning in &file.warnings {
                diagnostics.push(create_diagnostic_from_warning(warning));
            }

            // Process Error nodes in the AST (includes unknown syntax)
            for node in &file.nodes {
                if let ASTNode::Error(error_node) = node {
                    // Check if this is likely unknown syntax from a newer version
                    if is_unknown_syntax_warning(&error_node.message) {
                        diagnostics.push(create_warning_from_error_node(error_node));
                    } else {
                        diagnostics.push(create_diagnostic_from_error_node(error_node));
                    }
                }
            }

            // Process other parse errors (but skip ones already in error nodes)
            // Filter out errors that match error node messages to avoid duplicates
            let error_node_messages: Vec<String> = file
                .nodes
                .iter()
                .filter_map(|n| {
                    if let ASTNode::Error(e) = n {
                        Some(e.message.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for error in &file.errors {
                // Skip if this error was already reported via an error node
                if !error_node_messages
                    .iter()
                    .any(|msg| msg.contains(&error.message) || error.message.contains(msg))
                {
                    diagnostics.push(create_diagnostic_from_parse_error(error));
                }
            }

            // Perform semantic analysis on valid nodes only
            // Skip error nodes to allow rest of file to be analyzed
            let semantic_diagnostics = analyze_semantics(&file.nodes);
            diagnostics.extend(semantic_diagnostics);
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

fn create_warning_from_error_node(error_node: &crate::parser::ErrorNode) -> Diagnostic {
    let span = &error_node.span;

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
        severity: Some(DiagnosticSeverity::WARNING),
        code: None,
        code_description: None,
        source: Some("hone".to_string()),
        message: format!(
            "{} (This may be valid syntax in a newer version of Hone)",
            error_node.message
        ),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn is_unknown_syntax_warning(message: &str) -> bool {
    // Detect patterns that might be from newer versions
    // For now, treat "Unknown statement" as potentially new syntax
    message.starts_with("Unknown statement:")
}

fn analyze_semantics(nodes: &[ASTNode]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut in_test = false;

    for node in nodes {
        // Skip error nodes - they've already been reported
        // This allows us to continue analyzing the rest of the file
        if matches!(node, ASTNode::Error(_)) {
            continue;
        }

        match node {
            ASTNode::Test(_) => {
                in_test = true;
            }
            ASTNode::Assert(assert_node) => {
                // RUN and ASSERT outside TEST blocks are valid - the runner groups
                // them into an implicit test block, so no warning is needed.

                // Type check assertion arguments
                let type_diagnostics = check_assertion_types(assert_node);
                diagnostics.extend(type_diagnostics);
            }
            ASTNode::Run(_) => {
                // RUN outside TEST blocks is valid - forms an implicit test block
            }
            ASTNode::Env(_) => {
                if in_test {
                    diagnostics.push(create_semantic_diagnostic(
                        node.line(),
                        "Environment variable 'env' should be defined in @setup or at the top level, not inside @test",
                    ));
                }
            }
            _ => {}
        }
    }

    diagnostics
}

fn check_assertion_types(assert_node: &crate::parser::AssertNode) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    match &assert_node.expression {
        AssertionExpression::Output { predicate, .. } => match predicate {
            OutputPredicate::Contains { value } => {
                // `contains ""` is nonsensical - every string contains empty string
                if value.value.is_empty() {
                    diagnostics.push(create_semantic_diagnostic(
                        assert_node.line,
                        "String comparison value cannot be empty for 'contains'",
                    ));
                }
            }
            OutputPredicate::Equals { .. } => {
                // `== ""` is valid - it checks for empty output
            }
            OutputPredicate::Matches { value } => {
                if value.pattern.is_empty() {
                    diagnostics.push(create_semantic_diagnostic(
                        assert_node.line,
                        "Regex pattern cannot be empty",
                    ));
                } else {
                    // Validate regex syntax
                    let pattern = if value.flags.is_empty() {
                        value.pattern.clone()
                    } else {
                        format!("(?{}){}", value.flags, value.pattern)
                    };
                    if let Err(e) = regex::Regex::new(&pattern) {
                        diagnostics.push(create_semantic_diagnostic(
                            assert_node.line,
                            &format!("Invalid regex pattern: {}", e),
                        ));
                    }
                }
            }
        },
        AssertionExpression::ExitCode { predicate, .. } => {
            if predicate.value < 0 {
                diagnostics.push(create_semantic_diagnostic(
                    assert_node.line,
                    "Exit code must be a non-negative integer (0-255)",
                ));
            } else if predicate.value > 255 {
                diagnostics.push(create_semantic_diagnostic(
                    assert_node.line,
                    "Exit code must be in the range 0-255. Note: exit codes wrap around (256 becomes 0)",
                ));
            }
        }
        AssertionExpression::Duration { predicate, .. } => {
            if predicate.value.value < 0.0 {
                diagnostics.push(create_semantic_diagnostic(
                    assert_node.line,
                    "Duration value must be non-negative",
                ));
            }
            if predicate.value.value == 0.0
                && !matches!(
                    predicate.operator,
                    ComparisonOperator::GreaterThan
                        | ComparisonOperator::GreaterThanOrEqual
                        | ComparisonOperator::Equal
                        | ComparisonOperator::NotEqual
                )
            {
                diagnostics.push(create_semantic_diagnostic(
                    assert_node.line,
                    "Duration value of 0 may produce unexpected results",
                ));
            }
        }
        AssertionExpression::File { path, predicate } => {
            if path.value.is_empty() {
                diagnostics.push(create_semantic_diagnostic(
                    assert_node.line,
                    "File path cannot be empty",
                ));
            }

            match predicate {
                FilePredicate::Contains { value } => {
                    // `contains ""` is nonsensical - every string contains empty string
                    if value.value.is_empty() {
                        diagnostics.push(create_semantic_diagnostic(
                            assert_node.line,
                            "String comparison value cannot be empty for 'contains'",
                        ));
                    }
                }
                FilePredicate::Equals { .. } => {
                    // `== ""` is valid - it checks for empty file content
                }
                FilePredicate::Matches { value } => {
                    if value.pattern.is_empty() {
                        diagnostics.push(create_semantic_diagnostic(
                            assert_node.line,
                            "Regex pattern cannot be empty",
                        ));
                    } else {
                        // Validate regex syntax
                        let pattern = if value.flags.is_empty() {
                            value.pattern.clone()
                        } else {
                            format!("(?{}){}", value.flags, value.pattern)
                        };
                        if let Err(e) = regex::Regex::new(&pattern) {
                            diagnostics.push(create_semantic_diagnostic(
                                assert_node.line,
                                &format!("Invalid regex pattern: {}", e),
                            ));
                        }
                    }
                }
                FilePredicate::Exists => {
                    // No type checking needed for exists
                }
            }
        }
    }

    diagnostics
}

fn create_semantic_diagnostic(line: usize, message: &str) -> Diagnostic {
    let line = if line > 0 { line - 1 } else { 0 };

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
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("hone".to_string()),
        message: message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_too_large() {
        let content = r#"TEST "test"
RUN true
ASSERT exit_code == 300"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let exit_code_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Exit code must be in the range 0-255"))
            .collect();

        assert_eq!(exit_code_errors.len(), 1);
    }

    #[test]
    fn test_exit_code_negative() {
        let content = r#"TEST "test"
RUN true
ASSERT exit_code == -1"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let exit_code_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.message
                    .contains("Exit code must be a non-negative integer")
            })
            .collect();

        assert_eq!(exit_code_errors.len(), 1);
    }

    #[test]
    fn test_negative_duration() {
        let content = r#"TEST "test"
RUN true
ASSERT duration >= -100ms"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        // The parser itself rejects negative durations, so we should have a parse error instead
        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Expected duration value"))
            .collect();

        assert_eq!(parse_errors.len(), 1);
    }

    #[test]
    fn test_empty_file_path() {
        let content = r#"TEST "test"
RUN true
ASSERT file "" exists"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let file_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("File path cannot be empty"))
            .collect();

        assert_eq!(file_errors.len(), 1);
    }

    #[test]
    fn test_valid_assertions_no_errors() {
        let content = r#"TEST "test"
RUN true
ASSERT exit_code == 0
ASSERT duration >= 0ms
ASSERT stdout == "test"
ASSERT file "test.txt" exists"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        // Should not have any type errors
        let type_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.message.contains("Exit code")
                    || d.message.contains("Duration value")
                    || d.message.contains("File path")
            })
            .collect();

        assert_eq!(type_errors.len(), 0);
    }

    #[test]
    fn test_duration_less_than_zero_warning() {
        // `ASSERT duration < 0ms` will always fail since durations can't be negative
        let content = r#"TEST "test"
RUN true
ASSERT duration < 0ms"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let duration_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Duration value of 0"))
            .collect();

        assert_eq!(
            duration_warnings.len(),
            1,
            "ASSERT duration < 0ms should produce a warning"
        );
    }

    #[test]
    fn test_unknown_syntax_warning() {
        let content = r#"TEST "test"
RUN true
ASSERT exit_code == 0
UNKNOWN_STATEMENT foo bar
ASSERT stdout == "ok""#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        // Should have a warning for unknown syntax
        let unknown_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.message.contains("Unknown statement")
            })
            .collect();

        assert_eq!(unknown_warnings.len(), 1);
        assert!(unknown_warnings[0]
            .message
            .contains("newer version of Hone"));
    }

    #[test]
    fn test_graceful_degradation_continues_analysis() {
        let content = r#"TEST "test"
RUN true
UNKNOWN_SYNTAX_HERE
ASSERT exit_code == 300"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        // Should have warning for unknown syntax (not error)
        let unknown_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.message.contains("Unknown statement")
            })
            .collect();
        assert_eq!(unknown_warnings.len(), 1);

        // Should ALSO have error for invalid exit code (analysis continued)
        let exit_code_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Exit code must be in the range 0-255"))
            .collect();
        assert_eq!(exit_code_errors.len(), 1);
    }

    #[test]
    fn test_multiple_unknown_blocks_graceful() {
        let content = r#"TEST "test1"
RUN true
ASSERT exit_code == 0

FUTURE_BLOCK
some content

TEST "test2"
RUN false
ASSERT exit_code == 1"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        // Should have warnings for unknown syntax
        let unknown_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.message.contains("Unknown statement")
            })
            .collect();

        // Both FUTURE_BLOCK and "some content" should be unknown
        assert!(!unknown_warnings.is_empty());

        // Should still be able to analyze valid test blocks
        // No semantic errors should appear for correctly placed assertions
        let semantic_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.severity == Some(DiagnosticSeverity::ERROR)
                    && (d.message.contains("can only be used inside")
                        || d.message.contains("should be defined in"))
            })
            .collect();
        assert_eq!(semantic_errors.len(), 0);
    }

    #[test]
    fn test_invalid_regex_stdout_diagnostic() {
        let content = r#"TEST "regex test"
RUN echo hello
ASSERT stdout matches /[/"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let regex_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Invalid regex pattern"))
            .collect();
        assert_eq!(regex_errors.len(), 1, "Expected 1 invalid regex diagnostic");
    }

    #[test]
    fn test_invalid_regex_file_diagnostic() {
        let content = r#"TEST "file regex test"
RUN echo hello
ASSERT file "test.txt" matches /(?invalid)/"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let regex_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Invalid regex pattern"))
            .collect();
        assert_eq!(
            regex_errors.len(),
            1,
            "Expected 1 invalid regex diagnostic for file assertion"
        );
    }

    #[test]
    fn test_valid_regex_no_diagnostic() {
        let content = r#"TEST "valid regex"
RUN echo hello
ASSERT stdout matches /^hello$/"#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let regex_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Invalid regex pattern"))
            .collect();
        assert_eq!(
            regex_errors.len(),
            0,
            "Expected no invalid regex diagnostics for valid pattern"
        );
    }

    #[test]
    fn test_empty_string_equals_is_valid() {
        // `ASSERT stdout == ""` is valid - it checks for empty output
        let content = r#"TEST "empty output check"
RUN true
ASSERT stdout == """#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let empty_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("cannot be empty"))
            .collect();
        assert_eq!(
            empty_errors.len(),
            0,
            "ASSERT stdout == \"\" should be valid for checking empty output"
        );
    }

    #[test]
    fn test_empty_string_contains_is_invalid() {
        // `ASSERT stdout contains ""` is nonsensical - every string contains empty
        let content = r#"TEST "empty contains check"
RUN true
ASSERT stdout contains """#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let empty_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("cannot be empty"))
            .collect();
        assert_eq!(
            empty_errors.len(),
            1,
            "ASSERT stdout contains \"\" should produce a diagnostic"
        );
    }

    #[test]
    fn test_file_empty_string_contains_is_invalid() {
        // `ASSERT file "x" contains ""` is nonsensical - every string contains empty
        // This should produce the same diagnostic as `ASSERT stdout contains ""`
        let content = r#"TEST "empty file contains check"
RUN true
ASSERT file "test.txt" contains """#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let empty_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("cannot be empty"))
            .collect();
        assert_eq!(
            empty_errors.len(),
            1,
            "ASSERT file \"x\" contains \"\" should produce a diagnostic"
        );
    }

    #[test]
    fn test_run_and_assert_outside_test_block_is_valid() {
        // RUN and ASSERT outside TEST blocks are valid - they form an implicit test block
        // This is a valid pattern supported by the runner
        let content = r#"#! shell: /bin/bash
RUN echo hello
ASSERT stdout contains "hello""#;

        let diagnostics = generate_diagnostics(&Url::parse("file:///test.hone").unwrap(), content);

        let outside_test_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("can only be used inside"))
            .collect();
        assert_eq!(
            outside_test_errors.len(),
            0,
            "RUN and ASSERT outside TEST blocks should not produce diagnostics - this is valid syntax"
        );
    }
}
