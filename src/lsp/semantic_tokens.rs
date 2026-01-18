use async_lsp::lsp_types::*;

use crate::parser::{parse_file, ASTNode, ParseResult};

#[derive(Debug, Clone)]
pub struct SemanticTokensProvider {
    /// Token types legend (index = token type)
    token_types: Vec<SemanticTokenType>,
    /// Token modifiers legend (bit position = modifier)
    token_modifiers: Vec<SemanticTokenModifier>,
}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        Self {
            token_types: vec![
                SemanticTokenType::KEYWORD,
                SemanticTokenType::STRING,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::MACRO,
                SemanticTokenType::COMMENT,
                SemanticTokenType::NUMBER,
                SemanticTokenType::OPERATOR,
                SemanticTokenType::VARIABLE,
            ],
            token_modifiers: vec![
                SemanticTokenModifier::DECLARATION,
                SemanticTokenModifier::DEFINITION,
            ],
        }
    }

    pub fn legend(&self) -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: self.token_types.clone(),
            token_modifiers: self.token_modifiers.clone(),
        }
    }

    fn token_type_index(&self, token_type: &SemanticTokenType) -> u32 {
        self.token_types
            .iter()
            .position(|t| t == token_type)
            .unwrap_or(0) as u32
    }

    pub fn provide_semantic_tokens(&self, uri: &Url, text: &str) -> Option<SemanticTokensResult> {
        tracing::debug!("Providing semantic tokens for {}", uri);

        let parsed = match parse_file(text, uri.path()) {
            ParseResult::Success { file } => file,
            ParseResult::Failure { errors, .. } => {
                tracing::error!("Failed to parse file for semantic tokens: {:?}", errors);
                return None;
            }
        };

        let mut tokens = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        // Track previous token position for delta encoding
        let mut prev_line = 0;
        let mut prev_start = 0;

        // Process pragmas
        for pragma in &parsed.pragmas {
            if let Some((line, start, length)) = Self::find_token_in_line(
                &lines,
                pragma.line.saturating_sub(1),
                &format!("#{}", pragma.raw.trim_start_matches('#').trim()),
            ) {
                let (delta_line, delta_start) = if line == prev_line {
                    (0, start.saturating_sub(prev_start))
                } else {
                    (line.saturating_sub(prev_line), start)
                };

                tokens.push(SemanticToken {
                    delta_line: delta_line as u32,
                    delta_start: delta_start as u32,
                    length: length as u32,
                    token_type: self.token_type_index(&SemanticTokenType::KEYWORD),
                    token_modifiers_bitset: 0,
                });

                prev_line = line;
                prev_start = start;
            }
        }

        // Process AST nodes
        for node in &parsed.nodes {
            match node {
                ASTNode::Comment(comment_node) => {
                    let line_idx = comment_node.line.saturating_sub(1);
                    if line_idx < lines.len() {
                        let line = lines[line_idx];
                        if let Some(start) = line.find("//") {
                            let length = line[start..].len();
                            let (delta_line, delta_start) = if line_idx == prev_line {
                                (0, start.saturating_sub(prev_start))
                            } else {
                                (line_idx.saturating_sub(prev_line), start)
                            };

                            tokens.push(SemanticToken {
                                delta_line: delta_line as u32,
                                delta_start: delta_start as u32,
                                length: length as u32,
                                token_type: self.token_type_index(&SemanticTokenType::COMMENT),
                                token_modifiers_bitset: 0,
                            });

                            prev_line = line_idx;
                            prev_start = start;
                        }
                    }
                }
                ASTNode::Test(test_node) => {
                    let line_idx = test_node.line.saturating_sub(1);
                    if let Some((line, start, length)) =
                        Self::find_token_in_line(&lines, line_idx, "@test")
                    {
                        let (delta_line, delta_start) = if line == prev_line {
                            (0, start.saturating_sub(prev_start))
                        } else {
                            (line.saturating_sub(prev_line), start)
                        };

                        tokens.push(SemanticToken {
                            delta_line: delta_line as u32,
                            delta_start: delta_start as u32,
                            length: length as u32,
                            token_type: self.token_type_index(&SemanticTokenType::KEYWORD),
                            token_modifiers_bitset: 1, // DECLARATION
                        });

                        prev_line = line;
                        prev_start = start;

                        // Tokenize the test name string
                        if let Some(name_start) = lines[line_idx][start + length..].find('"') {
                            let name_pos = start + length + name_start;
                            let remaining = &lines[line_idx][name_pos..];
                            if let Some(name_end) = remaining[1..].find('"') {
                                let name_length = name_end + 2; // Include both quotes

                                let (delta_line, delta_start) = if line == prev_line {
                                    (0, name_pos.saturating_sub(prev_start))
                                } else {
                                    (line.saturating_sub(prev_line), name_pos)
                                };

                                tokens.push(SemanticToken {
                                    delta_line: delta_line as u32,
                                    delta_start: delta_start as u32,
                                    length: name_length as u32,
                                    token_type: self.token_type_index(&SemanticTokenType::STRING),
                                    token_modifiers_bitset: 0,
                                });

                                prev_line = line;
                                prev_start = name_pos;
                            }
                        }
                    }
                }
                ASTNode::Run(run_node) => {
                    let line_idx = run_node.line.saturating_sub(1);
                    if let Some((line, start, length)) =
                        Self::find_token_in_line(&lines, line_idx, "run")
                    {
                        let (delta_line, delta_start) = if line == prev_line {
                            (0, start.saturating_sub(prev_start))
                        } else {
                            (line.saturating_sub(prev_line), start)
                        };

                        tokens.push(SemanticToken {
                            delta_line: delta_line as u32,
                            delta_start: delta_start as u32,
                            length: length as u32,
                            token_type: self.token_type_index(&SemanticTokenType::KEYWORD),
                            token_modifiers_bitset: 0,
                        });

                        prev_line = line;
                        prev_start = start;

                        // If there's a named run, tokenize the name
                        if run_node.name.is_some() {
                            if let Some(name_start) = lines[line_idx][start + length..].find('"') {
                                let name_pos = start + length + name_start;
                                let remaining = &lines[line_idx][name_pos..];
                                if let Some(name_end) = remaining[1..].find('"') {
                                    let name_length = name_end + 2;

                                    let (delta_line, delta_start) = if line == prev_line {
                                        (0, name_pos.saturating_sub(prev_start))
                                    } else {
                                        (line.saturating_sub(prev_line), name_pos)
                                    };

                                    tokens.push(SemanticToken {
                                        delta_line: delta_line as u32,
                                        delta_start: delta_start as u32,
                                        length: name_length as u32,
                                        token_type: self
                                            .token_type_index(&SemanticTokenType::STRING),
                                        token_modifiers_bitset: 0,
                                    });

                                    prev_line = line;
                                    prev_start = name_pos;
                                }
                            }
                        }

                        // Tokenize the shell command as a macro
                        if let Some(brace_start) = lines[line_idx][start + length..].find('{') {
                            let cmd_line_idx = line_idx;
                            let cmd_start = start + length + brace_start + 1;
                            if cmd_line_idx < lines.len() {
                                let cmd_line = &lines[cmd_line_idx][cmd_start..];
                                let cmd_end = cmd_line.find('}').unwrap_or(cmd_line.len());
                                let cmd_length = cmd_end;

                                if cmd_length > 0 {
                                    let (delta_line, delta_start) = if cmd_line_idx == prev_line {
                                        (0, cmd_start.saturating_sub(prev_start))
                                    } else {
                                        (cmd_line_idx.saturating_sub(prev_line), cmd_start)
                                    };

                                    tokens.push(SemanticToken {
                                        delta_line: delta_line as u32,
                                        delta_start: delta_start as u32,
                                        length: cmd_length as u32,
                                        token_type: self
                                            .token_type_index(&SemanticTokenType::MACRO),
                                        token_modifiers_bitset: 0,
                                    });

                                    prev_line = cmd_line_idx;
                                    prev_start = cmd_start;
                                }
                            }
                        }
                    }
                }
                ASTNode::Assert(assert_node) => {
                    let line_idx = assert_node.line.saturating_sub(1);
                    if let Some((line, start, length)) =
                        Self::find_token_in_line(&lines, line_idx, "expect")
                    {
                        let (delta_line, delta_start) = if line == prev_line {
                            (0, start.saturating_sub(prev_start))
                        } else {
                            (line.saturating_sub(prev_line), start)
                        };

                        tokens.push(SemanticToken {
                            delta_line: delta_line as u32,
                            delta_start: delta_start as u32,
                            length: length as u32,
                            token_type: self.token_type_index(&SemanticTokenType::KEYWORD),
                            token_modifiers_bitset: 0,
                        });

                        prev_line = line;
                        prev_start = start;

                        // Tokenize the assertion name (stdout, stderr, exitcode, etc.)
                        let assertion_names = [
                            "stdout",
                            "stdout_raw",
                            "stderr",
                            "exitcode",
                            "duration",
                            "file",
                        ];
                        for assertion_name in &assertion_names {
                            if let Some((line, start, length)) =
                                Self::find_token_in_line(&lines, line_idx, assertion_name)
                            {
                                // Make sure this token comes after "expect"
                                if start > prev_start || line > prev_line {
                                    let (delta_line, delta_start) = if line == prev_line {
                                        (0, start.saturating_sub(prev_start))
                                    } else {
                                        (line.saturating_sub(prev_line), start)
                                    };

                                    tokens.push(SemanticToken {
                                        delta_line: delta_line as u32,
                                        delta_start: delta_start as u32,
                                        length: length as u32,
                                        token_type: self
                                            .token_type_index(&SemanticTokenType::FUNCTION),
                                        token_modifiers_bitset: 0,
                                    });

                                    prev_line = line;
                                    prev_start = start;
                                    break;
                                }
                            }
                        }
                    }
                }
                ASTNode::Env(env_node) => {
                    let line_idx = env_node.line.saturating_sub(1);
                    if let Some((line, start, length)) =
                        Self::find_token_in_line(&lines, line_idx, "env")
                    {
                        let (delta_line, delta_start) = if line == prev_line {
                            (0, start.saturating_sub(prev_start))
                        } else {
                            (line.saturating_sub(prev_line), start)
                        };

                        tokens.push(SemanticToken {
                            delta_line: delta_line as u32,
                            delta_start: delta_start as u32,
                            length: length as u32,
                            token_type: self.token_type_index(&SemanticTokenType::KEYWORD),
                            token_modifiers_bitset: 0,
                        });

                        prev_line = line;
                        prev_start = start;
                    }
                }
                ASTNode::Pragma(_) => {
                    // Already handled above
                }
                ASTNode::Error(_) => {
                    // Don't provide tokens for error nodes
                }
            }
        }

        Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        }))
    }

    fn find_token_in_line(
        lines: &[&str],
        line_idx: usize,
        token: &str,
    ) -> Option<(usize, usize, usize)> {
        if line_idx < lines.len() {
            let line = lines[line_idx];
            if let Some(pos) = line.find(token) {
                return Some((line_idx, pos, token.len()));
            }
        }
        None
    }
}

impl Default for SemanticTokensProvider {
    fn default() -> Self {
        Self::new()
    }
}
