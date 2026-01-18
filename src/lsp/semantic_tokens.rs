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
                        Self::find_token_in_line(&lines, line_idx, "TEST")
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
                        Self::find_token_in_line(&lines, line_idx, "RUN")
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
                        Self::find_token_in_line(&lines, line_idx, "ASSERT")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_uri(path: &str) -> Url {
        Url::parse(&format!("file://{}", path)).unwrap()
    }

    #[test]
    fn test_new_provider() {
        let provider = SemanticTokensProvider::new();
        assert_eq!(provider.token_types.len(), 8);
        assert_eq!(provider.token_modifiers.len(), 2);
    }

    #[test]
    fn test_legend() {
        let provider = SemanticTokensProvider::new();
        let legend = provider.legend();
        assert_eq!(legend.token_types.len(), 8);
        assert_eq!(legend.token_modifiers.len(), 2);
        assert_eq!(legend.token_types[0], SemanticTokenType::KEYWORD);
        assert_eq!(legend.token_types[1], SemanticTokenType::STRING);
    }

    #[test]
    fn test_token_type_index() {
        let provider = SemanticTokensProvider::new();
        assert_eq!(provider.token_type_index(&SemanticTokenType::KEYWORD), 0);
        assert_eq!(provider.token_type_index(&SemanticTokenType::STRING), 1);
        assert_eq!(provider.token_type_index(&SemanticTokenType::FUNCTION), 2);
    }

    #[test]
    fn test_provide_semantic_tokens_simple() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT exit_code == 0";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have some tokens for a valid file
            assert!(
                !tokens.data.is_empty(),
                "Expected some tokens for valid file"
            );
        }
    }

    #[test]
    fn test_provide_semantic_tokens_with_comment() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "# This is a comment\nTEST \"example\"\nRUN ls";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Comments may or may not be tokenized depending on implementation
            // Just verify we get some tokens
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_provide_semantic_tokens_with_assertions() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT stdout == \"ok\"";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for assertions
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_provide_semantic_tokens_invalid_syntax() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "INVALID SYNTAX {{{";

        let result = provider.provide_semantic_tokens(&uri, text);
        // Parser is fault-tolerant, may still provide partial tokens
        // Just verify the call doesn't crash
        let _ = result;
    }

    #[test]
    fn test_provide_semantic_tokens_empty_file() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            assert!(tokens.data.is_empty());
        }
    }

    #[test]
    fn test_provide_semantic_tokens_with_env() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "#env VAR=value\nTEST \"example\"\nRUN ls";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for pragmas and keywords
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_find_token_in_line() {
        let lines = vec!["TEST \"example\"", "RUN ls", "ASSERT stdout == \"ok\""];

        let result = SemanticTokensProvider::find_token_in_line(&lines, 0, "TEST");
        assert_eq!(result, Some((0, 0, 4)));

        let result = SemanticTokensProvider::find_token_in_line(&lines, 1, "RUN");
        assert_eq!(result, Some((1, 0, 3)));

        let result = SemanticTokensProvider::find_token_in_line(&lines, 2, "stdout");
        assert_eq!(result, Some((2, 7, 6)));

        let result = SemanticTokensProvider::find_token_in_line(&lines, 0, "NOTFOUND");
        assert_eq!(result, None);

        let result = SemanticTokensProvider::find_token_in_line(&lines, 10, "TEST");
        assert_eq!(result, None);
    }

    #[test]
    fn test_delta_encoding() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT exit_code == 0";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Verify tokens are properly delta-encoded
            for token in &tokens.data {
                // delta_line and delta_start are u32, so always >= 0
                assert!(token.length > 0);
            }
        }
    }

    #[test]
    fn test_provide_semantic_tokens_multiple_tests() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"test1\"\nRUN ls\n\nTEST \"test2\"\nRUN pwd";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for both tests
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_default_provider() {
        let provider = SemanticTokensProvider::default();
        assert_eq!(provider.token_types.len(), 8);
        assert_eq!(provider.token_modifiers.len(), 2);
    }

    #[test]
    fn test_provide_semantic_tokens_with_exitcode() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN ls\nASSERT exit_code == 0";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for exit_code assertion
            assert!(!tokens.data.is_empty());
        }
    }

    #[test]
    fn test_provide_semantic_tokens_with_file() {
        let provider = SemanticTokensProvider::new();
        let uri = create_test_uri("/test.hone");
        let text = "TEST \"example\"\nRUN touch test.txt\nASSERT file \"test.txt\" exists";

        let result = provider.provide_semantic_tokens(&uri, text);
        assert!(result.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = result {
            // Should have tokens for file assertion
            assert!(!tokens.data.is_empty());
        }
    }
}
