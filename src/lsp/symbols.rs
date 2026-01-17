use async_lsp::lsp_types::*;

use crate::parser::ast::{ASTNode, ParsedFile};

#[derive(Debug, Clone)]
pub struct SymbolsProvider;

impl Default for SymbolsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolsProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn provide_symbols(&self, parsed: &ParsedFile) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        let mut current_test_symbol: Option<DocumentSymbol> = None;

        for node in &parsed.nodes {
            match node {
                ASTNode::Test(test) => {
                    // Save previous test if exists
                    if let Some(test_sym) = current_test_symbol.take() {
                        symbols.push(test_sym);
                    }

                    // Create new test symbol
                    let line = test.line.saturating_sub(1) as u32;
                    let name_len = test.name.len() as u32;

                    current_test_symbol = Some(DocumentSymbol {
                        name: test.name.clone(),
                        detail: Some("test".to_string()),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position {
                                line,
                                character: name_len + 10, // Approximate for "@test "
                            },
                        },
                        selection_range: Range {
                            start: Position {
                                line,
                                character: 6, // After "@test "
                            },
                            end: Position {
                                line,
                                character: 6 + name_len,
                            },
                        },
                        children: Some(Vec::new()),
                        #[allow(deprecated)]
                        deprecated: None,
                    });
                }
                ASTNode::Pragma(pragma)
                    if pragma.pragma_type == crate::parser::ast::PragmaType::Shell =>
                {
                    // Setup block
                    if current_test_symbol.is_some() {
                        // Save previous test if exists
                        if let Some(test_sym) = current_test_symbol.take() {
                            symbols.push(test_sym);
                        }
                    }

                    let line = pragma.line.saturating_sub(1) as u32;

                    let setup_symbol = DocumentSymbol {
                        name: "@setup".to_string(),
                        detail: Some("setup block".to_string()),
                        kind: SymbolKind::CONSTRUCTOR,
                        tags: None,
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position {
                                line,
                                character: 20,
                            },
                        },
                        selection_range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 6 },
                        },
                        children: None,
                        #[allow(deprecated)]
                        deprecated: None,
                    };
                    symbols.push(setup_symbol);
                }
                ASTNode::Assert(assert) => {
                    // Add assertion as child of current test
                    if let Some(ref mut test_sym) = current_test_symbol {
                        let line = assert.line.saturating_sub(1) as u32;

                        // Extract a readable name from the assertion
                        let name = extract_assertion_name(assert);

                        let assert_symbol = DocumentSymbol {
                            name,
                            detail: Some("assertion".to_string()),
                            kind: SymbolKind::PROPERTY,
                            tags: None,
                            range: Range {
                                start: Position { line, character: 0 },
                                end: Position {
                                    line,
                                    character: 40, // Approximate
                                },
                            },
                            selection_range: Range {
                                start: Position { line, character: 0 },
                                end: Position {
                                    line,
                                    character: 6, // "expect"
                                },
                            },
                            children: None,
                            #[allow(deprecated)]
                            deprecated: None,
                        };

                        if let Some(ref mut children) = test_sym.children {
                            children.push(assert_symbol);
                        }
                    }
                }
                ASTNode::Run(run) => {
                    // Add run command as child of current test
                    if let Some(ref mut test_sym) = current_test_symbol {
                        let line = run.line.saturating_sub(1) as u32;

                        let name = if let Some(ref run_name) = run.name {
                            format!("run \"{}\"", run_name)
                        } else {
                            format!("run: {}", truncate_command(&run.command))
                        };

                        let run_symbol = DocumentSymbol {
                            name,
                            detail: Some("command".to_string()),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            range: Range {
                                start: Position { line, character: 0 },
                                end: Position {
                                    line,
                                    character: 30, // Approximate
                                },
                            },
                            selection_range: Range {
                                start: Position { line, character: 0 },
                                end: Position {
                                    line,
                                    character: 3, // "run"
                                },
                            },
                            children: None,
                            #[allow(deprecated)]
                            deprecated: None,
                        };

                        if let Some(ref mut children) = test_sym.children {
                            children.push(run_symbol);
                        }
                    }
                }
                _ => {}
            }
        }

        // Don't forget the last test
        if let Some(test_sym) = current_test_symbol {
            symbols.push(test_sym);
        }

        symbols
    }
}

fn extract_assertion_name(assert: &crate::parser::ast::AssertNode) -> String {
    use crate::parser::ast::AssertionExpression;

    match &assert.expression {
        AssertionExpression::Output {
            selector,
            predicate,
            ..
        } => {
            let selector_str = match selector {
                crate::parser::ast::OutputSelector::Stdout => "stdout",
                crate::parser::ast::OutputSelector::StdoutRaw => "stdout-raw",
                crate::parser::ast::OutputSelector::Stderr => "stderr",
            };

            let predicate_str = match predicate {
                crate::parser::ast::OutputPredicate::Contains { .. } => "contains",
                crate::parser::ast::OutputPredicate::Matches { .. } => "matches",
                crate::parser::ast::OutputPredicate::Equals { .. } => "equals",
            };

            format!("expect {} {}", selector_str, predicate_str)
        }
        AssertionExpression::ExitCode { .. } => "expect exitcode".to_string(),
        AssertionExpression::Duration { .. } => "expect duration".to_string(),
        AssertionExpression::File { path, .. } => {
            format!("expect file \"{}\"", path.value)
        }
    }
}

fn truncate_command(cmd: &str) -> String {
    const MAX_LEN: usize = 40;
    if cmd.len() > MAX_LEN {
        format!("{}...", &cmd[..MAX_LEN])
    } else {
        cmd.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_symbols_with_test_blocks() {
        let parsed = ParsedFile {
            filename: "test.hone".to_string(),
            pragmas: vec![],
            nodes: vec![
                ASTNode::Test(TestNode {
                    name: "my test".to_string(),
                    line: 1,
                }),
                ASTNode::Test(TestNode {
                    name: "another test".to_string(),
                    line: 5,
                }),
            ],
            warnings: vec![],
            errors: vec![],
        };

        let provider = SymbolsProvider::new();
        let symbols = provider.provide_symbols(&parsed);

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "my test");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
        assert_eq!(symbols[1].name, "another test");
    }

    #[test]
    fn test_symbols_with_nested_assertions() {
        let parsed = ParsedFile {
            filename: "test.hone".to_string(),
            pragmas: vec![],
            nodes: vec![
                ASTNode::Test(TestNode {
                    name: "my test".to_string(),
                    line: 1,
                }),
                ASTNode::Assert(AssertNode {
                    expression: AssertionExpression::ExitCode {
                        target: None,
                        predicate: ExitCodePredicate {
                            operator: StringComparisonOperator::Equal,
                            value: 0,
                        },
                    },
                    line: 2,
                    raw: "expect exitcode 0".to_string(),
                }),
            ],
            warnings: vec![],
            errors: vec![],
        };

        let provider = SymbolsProvider::new();
        let symbols = provider.provide_symbols(&parsed);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "my test");

        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "expect exitcode");
        assert_eq!(children[0].kind, SymbolKind::PROPERTY);
    }
}
