use crate::parser::ast::*;
use crate::parser::errors::ParseErrorCollector;
use crate::parser::lexer::*;
use std::collections::HashSet;
use std::sync::OnceLock;

pub fn parse_file(content: &str, filename: &str) -> ParseResult {
    let lines: Vec<&str> = content.lines().collect();
    let mut collector = ParseErrorCollector::new(filename.to_string());
    let mut pragmas: Vec<PragmaNode> = Vec::new();
    let mut nodes: Vec<ASTNode> = Vec::new();
    let mut run_names: HashSet<String> = HashSet::new();

    let mut in_pragma_section = true;

    for (i, line) in lines.iter().enumerate() {
        let line_number = i + 1;
        let token = classify_line(line, line_number);

        match token.token_type {
            TokenType::Empty => {
                // Skip empty lines
            }

            TokenType::Comment => {
                nodes.push(ASTNode::Comment(CommentNode {
                    text: token.content[1..].trim().to_string(),
                    line: line_number,
                }));
            }

            TokenType::Pragma => {
                if !in_pragma_section {
                    collector.add_error(
                        "Pragmas must appear at the top of the file".to_string(),
                        line_number,
                    );
                    continue;
                }

                if let Some(pragma) = parse_pragma(&token.content, line_number, &mut collector) {
                    pragmas.push(pragma.clone());
                    nodes.push(ASTNode::Pragma(pragma));
                }
            }

            TokenType::Test => {
                in_pragma_section = false;
                run_names.clear();
                if let Some(test) = parse_test(&token.content, line_number, &mut collector) {
                    nodes.push(ASTNode::Test(test));
                }
            }

            TokenType::Run => {
                in_pragma_section = false;
                if let Some(run) =
                    parse_run(&token.content, line_number, &mut collector, &mut run_names)
                {
                    nodes.push(ASTNode::Run(run));
                }
            }

            TokenType::Assert => {
                in_pragma_section = false;
                if let Some(assert) = parse_assert(&token.content, line_number, &mut collector) {
                    nodes.push(ASTNode::Assert(assert));
                }
            }

            TokenType::Env => {
                in_pragma_section = false;
                if let Some(env) = parse_env(&token.content, line_number, &mut collector) {
                    nodes.push(ASTNode::Env(env));
                }
            }

            TokenType::Unknown => {
                in_pragma_section = false;
                let span = Span::single_line(line_number, 0, line.len());
                nodes.push(ASTNode::Error(ErrorNode {
                    message: format!("Unknown statement: {}", token.content),
                    span,
                    raw: token.content.clone(),
                }));
                collector.add_error(format!("Unknown statement: {}", token.content), line_number);
            }

            TokenType::Error => {
                in_pragma_section = false;
                let span = Span::single_line(line_number, 0, line.len());
                nodes.push(ASTNode::Error(ErrorNode {
                    message: format!("Lexer error: {}", token.content),
                    span,
                    raw: token.content.clone(),
                }));
                collector.add_error(format!("Lexer error: {}", token.content), line_number);
            }
        }
    }

    // Always return Success with error nodes embedded in the AST
    // This enables LSP features on partial/invalid files
    ParseResult::Success {
        file: ParsedFile {
            filename: filename.to_string(),
            pragmas,
            nodes,
            warnings: collector.get_warnings(),
            errors: collector.get_errors(),
        },
    }
}

fn parse_pragma(
    content: &str,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<PragmaNode> {
    // Remove #! prefix
    let rest = content[2..].trim();

    // Parse key: value
    let Some(colon_index) = rest.find(':') else {
        // Missing colon - warn the user about the syntax error
        if rest.is_empty() {
            collector.add_warning("Invalid pragma syntax: empty pragma".to_string(), line);
        } else {
            collector.add_warning(
                format!(
                    "Invalid pragma syntax: expected 'key: value' format, got '{}'",
                    rest
                ),
                line,
            );
        }
        return None;
    };

    let pragma_key = rest[..colon_index].trim().to_lowercase();
    let pragma_value = rest[colon_index + 1..].trim();

    match pragma_key.as_str() {
        "shell" => Some(PragmaNode {
            pragma_type: PragmaType::Shell,
            key: None,
            value: pragma_value.to_string(),
            line,
            raw: content.to_string(),
        }),

        "env" => {
            // Parse KEY=value
            let Some(eq_index) = pragma_value.find('=') else {
                collector.add_error(format!("Invalid env pragma: {}", content), line);
                return None;
            };
            let env_key = pragma_value[..eq_index].trim().to_string();
            let env_value = pragma_value[eq_index + 1..].to_string();

            if env_key.is_empty() {
                collector.add_error("Invalid env pragma: empty key".to_string(), line);
                return None;
            }

            // Validate key format (must be valid environment variable name)
            static PRAGMA_ENV_KEY_RE: OnceLock<regex::Regex> = OnceLock::new();
            let re = PRAGMA_ENV_KEY_RE.get_or_init(|| {
                regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$")
                    .expect("pragma env key regex should be valid")
            });

            if !re.is_match(&env_key) {
                collector.add_error(
                    format!(
                        "Invalid environment variable name: \"{}\". Names must start with a letter or underscore and contain only alphanumeric characters and underscores",
                        env_key
                    ),
                    line,
                );
                return None;
            }

            Some(PragmaNode {
                pragma_type: PragmaType::Env,
                key: Some(env_key),
                value: env_value,
                line,
                raw: content.to_string(),
            })
        }

        "timeout" => {
            // Validate timeout format
            let Some((duration, _)) = parse_duration(pragma_value, 0) else {
                collector.add_error(
                    format!(
                        "Invalid timeout format: {}. Expected format: <number>s or <number>ms",
                        pragma_value
                    ),
                    line,
                );
                return None;
            };

            // Convert to milliseconds and validate minimum
            let ms_value = match duration.unit {
                crate::parser::ast::DurationUnit::Seconds => duration.value * 1000.0,
                crate::parser::ast::DurationUnit::Milliseconds => duration.value,
            };
            if ms_value < 1.0 {
                collector.add_error(
                    format!(
                        "Timeout value too small: {}. Minimum timeout is 1ms",
                        pragma_value
                    ),
                    line,
                );
                return None;
            }

            Some(PragmaNode {
                pragma_type: PragmaType::Timeout,
                key: None,
                value: pragma_value.to_string(),
                line,
                raw: content.to_string(),
            })
        }

        _ => {
            // Unknown pragma - warn but continue
            collector.add_warning(format!("Unknown pragma: {}", pragma_key), line);
            Some(PragmaNode {
                pragma_type: PragmaType::Unknown,
                key: None,
                value: rest.to_string(),
                line,
                raw: content.to_string(),
            })
        }
    }
}

fn parse_test(content: &str, line: usize, collector: &mut ParseErrorCollector) -> Option<TestNode> {
    // TEST "name"
    let rest = &content[5..]; // After "TEST "
    let Some(result) = parse_string_literal(rest, 0) else {
        collector.add_error("Expected quoted string after TEST".to_string(), line);
        return None;
    };

    let name = result.0.value.clone();

    if name.is_empty() {
        collector.add_error("Test name cannot be empty".to_string(), line);
        return None;
    }

    Some(TestNode { name, line })
}

fn parse_run(
    content: &str,
    line: usize,
    collector: &mut ParseErrorCollector,
    run_names: &mut HashSet<String>,
) -> Option<RunNode> {
    // RUN <command> or RUN <name>: <command>
    let rest = &content[4..]; // After "RUN "

    // Check for named RUN (name: command)
    static NAMED_RUN_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = NAMED_RUN_RE.get_or_init(|| {
        regex::Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*):\s*")
            .expect("named run regex should be valid")
    });

    if let Some(captures) = re.captures(rest) {
        if let (Some(name_match), Some(full_match)) = (captures.get(1), captures.get(0)) {
            let name = name_match.as_str().to_string();
            let matched_len = full_match.as_str().len();
            let command = rest[matched_len..].to_string();

            if run_names.contains(&name) {
                collector.add_error(
                    format!(
                        "Duplicate RUN name: \"{}\". RUN names must be unique within a test",
                        name
                    ),
                    line,
                );
                return None;
            }

            run_names.insert(name.clone());

            if command.trim().is_empty() {
                collector.add_error("Empty command in RUN statement".to_string(), line);
                return None;
            }

            return Some(RunNode {
                name: Some(name),
                command: command.trim().to_string(),
                line,
            });
        }
    }

    // Unnamed RUN
    if rest.trim().is_empty() {
        collector.add_error("Empty command in RUN statement".to_string(), line);
        return None;
    }

    Some(RunNode {
        name: None,
        command: rest.trim().to_string(),
        line,
    })
}

fn parse_env(content: &str, line: usize, collector: &mut ParseErrorCollector) -> Option<EnvNode> {
    // ENV KEY=value
    let rest = &content[4..]; // After "ENV "
    let Some(eq_index) = rest.find('=') else {
        collector.add_error(
            "Invalid ENV syntax: expected KEY=value format".to_string(),
            line,
        );
        return None;
    };

    let key = rest[..eq_index].trim().to_string();
    let value = rest[eq_index + 1..].to_string();

    if key.is_empty() {
        collector.add_error("Invalid ENV syntax: empty key".to_string(), line);
        return None;
    }

    // Validate key format (valid environment variable name)
    static ENV_KEY_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = ENV_KEY_RE.get_or_init(|| {
        regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").expect("env key regex should be valid")
    });

    if !re.is_match(&key) {
        collector.add_error(
            format!(
                "Invalid environment variable name: \"{}\". Names must start with a letter or underscore and contain only alphanumeric characters and underscores",
                key
            ),
            line,
        );
        return None;
    }

    Some(EnvNode { key, value, line })
}

fn parse_assert(
    content: &str,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertNode> {
    // ASSERT <expression>
    let rest = &content[7..]; // After "ASSERT "
    let expression = parse_assertion_expression(rest, line, collector)?;

    Some(AssertNode {
        expression,
        line,
        raw: content.to_string(),
    })
}

fn parse_assertion_expression(
    input: &str,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertionExpression> {
    let mut i = skip_whitespace(input, 0);

    // Check for file assertion
    if match_word(input, i, "file") {
        return parse_file_assertion(input, line, collector);
    }

    // Check for stdout.raw first
    if match_word(input, i, "stdout.raw") {
        i += 10; // "stdout.raw"
        return parse_output_assertion(input, i, OutputSelector::StdoutRaw, None, line, collector);
    }

    // Check for named target (e.g., build.stdout)
    let mut target: Option<String> = None;
    static NAMED_TARGET_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = NAMED_TARGET_RE.get_or_init(|| {
        regex::Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)\.(.+)")
            .expect("named target regex should be valid")
    });

    let mut effective_input = input;
    if let Some(captures) = re.captures(input) {
        if let (Some(potential_target_match), Some(remainder_match)) =
            (captures.get(1), captures.get(2))
        {
            let potential_target = potential_target_match.as_str();
            if potential_target != "stdout"
                && potential_target != "stderr"
                && potential_target != "exit_code"
                && potential_target != "duration"
            {
                target = Some(potential_target.to_string());
                effective_input = remainder_match.as_str();
                i = 0;
            }
        }
    }

    // Parse selector
    if match_word(effective_input, i, "stdout.raw") {
        i += 10;
        return parse_output_assertion(
            effective_input,
            i,
            OutputSelector::StdoutRaw,
            target,
            line,
            collector,
        );
    }

    if match_word(effective_input, i, "stdout") {
        i += 6;
        return parse_output_assertion(
            effective_input,
            i,
            OutputSelector::Stdout,
            target,
            line,
            collector,
        );
    }

    if match_word(effective_input, i, "stderr") {
        i += 6;
        return parse_output_assertion(
            effective_input,
            i,
            OutputSelector::Stderr,
            target,
            line,
            collector,
        );
    }

    if match_word(effective_input, i, "exit_code") {
        i += 9;
        return parse_exit_code_assertion(effective_input, i, target, line, collector);
    }

    if match_word(effective_input, i, "duration") {
        i += 8;
        return parse_duration_assertion(effective_input, i, target, line, collector);
    }

    collector.add_error(format!("Unknown assertion type: {}", input), line);
    None
}

fn parse_output_assertion(
    input: &str,
    start_index: usize,
    selector: OutputSelector,
    target: Option<String>,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertionExpression> {
    let mut i = skip_whitespace(input, start_index);

    // Parse predicate
    if match_word(input, i, "contains") {
        i += 8;
        i = skip_whitespace(input, i);

        let Some((string_lit, _)) = parse_string_literal(input, i) else {
            collector.add_error(
                "Expected quoted string after \"contains\"".to_string(),
                line,
            );
            return None;
        };

        return Some(AssertionExpression::Output {
            target,
            selector,
            predicate: OutputPredicate::Contains { value: string_lit },
        });
    }

    if match_word(input, i, "matches") {
        i += 7;
        i = skip_whitespace(input, i);

        let Some((regex_lit, _)) = parse_regex_literal(input, i) else {
            collector.add_error("Expected regex literal after \"matches\"".to_string(), line);
            return None;
        };

        return Some(AssertionExpression::Output {
            target,
            selector,
            predicate: OutputPredicate::Matches { value: regex_lit },
        });
    }

    // Check for == or !=
    if let Some((op, end_index)) = parse_comparison_operator(input, i) {
        if matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
            i = skip_whitespace(input, end_index);

            let Some((string_lit, _)) = parse_string_literal(input, i) else {
                collector.add_error(
                    "Expected quoted string after comparison operator".to_string(),
                    line,
                );
                return None;
            };

            let string_op = match op {
                ComparisonOperator::Equal => StringComparisonOperator::Equal,
                ComparisonOperator::NotEqual => StringComparisonOperator::NotEqual,
                _ => unreachable!(),
            };

            return Some(AssertionExpression::Output {
                target,
                selector,
                predicate: OutputPredicate::Equals {
                    operator: string_op,
                    value: string_lit,
                },
            });
        }
    }

    let selector_str = match selector {
        OutputSelector::Stdout => "stdout",
        OutputSelector::StdoutRaw => "stdout.raw",
        OutputSelector::Stderr => "stderr",
    };

    collector.add_error(
        format!(
            "Expected predicate (contains, matches, ==, !=) after \"{}\"",
            selector_str
        ),
        line,
    );
    None
}

fn parse_exit_code_assertion(
    input: &str,
    start_index: usize,
    target: Option<String>,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertionExpression> {
    let Some((op, end_index)) = parse_comparison_operator(input, start_index) else {
        collector.add_error("Expected == or != after \"exit_code\"".to_string(), line);
        return None;
    };

    if !matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
        collector.add_error("Expected == or != after \"exit_code\"".to_string(), line);
        return None;
    }

    let num_value = match parse_number_checked(input, end_index) {
        ParseNumberResult::Success(value, _) => value,
        ParseNumberResult::Overflow => {
            collector.add_error(
                "Exit code value is too large (must fit in i32 range)".to_string(),
                line,
            );
            return None;
        }
        ParseNumberResult::NotANumber => {
            collector.add_error(
                "Expected number after comparison operator".to_string(),
                line,
            );
            return None;
        }
    };

    let string_op = match op {
        ComparisonOperator::Equal => StringComparisonOperator::Equal,
        ComparisonOperator::NotEqual => StringComparisonOperator::NotEqual,
        _ => unreachable!(),
    };

    Some(AssertionExpression::ExitCode {
        target,
        predicate: ExitCodePredicate {
            operator: string_op,
            value: num_value,
        },
    })
}

fn parse_duration_assertion(
    input: &str,
    start_index: usize,
    target: Option<String>,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertionExpression> {
    let Some((op, end_index)) = parse_comparison_operator(input, start_index) else {
        collector.add_error(
            "Expected comparison operator after \"duration\"".to_string(),
            line,
        );
        return None;
    };

    let Some((duration_value, _)) = parse_duration(input, end_index) else {
        collector.add_error(
            "Expected duration value (e.g., 200ms, 1.5s) after comparison operator".to_string(),
            line,
        );
        return None;
    };

    Some(AssertionExpression::Duration {
        target,
        predicate: DurationPredicate {
            operator: op,
            value: duration_value,
        },
    })
}

fn parse_file_assertion(
    input: &str,
    line: usize,
    collector: &mut ParseErrorCollector,
) -> Option<AssertionExpression> {
    // file "path" <predicate>
    let mut i = 4; // After "file"
    i = skip_whitespace(input, i);

    let Some((path, end_index)) = parse_string_literal(input, i) else {
        collector.add_error("Expected quoted file path after \"file\"".to_string(), line);
        return None;
    };
    i = skip_whitespace(input, end_index);

    // Parse predicate
    if match_word(input, i, "exists") {
        return Some(AssertionExpression::File {
            path,
            predicate: FilePredicate::Exists,
        });
    }

    if match_word(input, i, "contains") {
        i += 8;
        i = skip_whitespace(input, i);

        let Some((string_lit, _)) = parse_string_literal(input, i) else {
            collector.add_error(
                "Expected quoted string after \"contains\"".to_string(),
                line,
            );
            return None;
        };

        return Some(AssertionExpression::File {
            path,
            predicate: FilePredicate::Contains { value: string_lit },
        });
    }

    if match_word(input, i, "matches") {
        i += 7;
        i = skip_whitespace(input, i);

        let Some((regex_lit, _)) = parse_regex_literal(input, i) else {
            collector.add_error("Expected regex literal after \"matches\"".to_string(), line);
            return None;
        };

        return Some(AssertionExpression::File {
            path,
            predicate: FilePredicate::Matches { value: regex_lit },
        });
    }

    // Check for == or !=
    if let Some((op, end_index)) = parse_comparison_operator(input, i) {
        if matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
            i = skip_whitespace(input, end_index);

            let Some((string_lit, _)) = parse_string_literal(input, i) else {
                collector.add_error(
                    "Expected quoted string after comparison operator".to_string(),
                    line,
                );
                return None;
            };

            let string_op = match op {
                ComparisonOperator::Equal => StringComparisonOperator::Equal,
                ComparisonOperator::NotEqual => StringComparisonOperator::NotEqual,
                _ => unreachable!(),
            };

            return Some(AssertionExpression::File {
                path,
                predicate: FilePredicate::Equals {
                    operator: string_op,
                    value: string_lit,
                },
            });
        }
    }

    collector.add_error(
        "Expected predicate (exists, contains, matches, ==, !=) after file path".to_string(),
        line,
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_name(name: &str) -> bool {
        let content = format!("TEST \"{}\"", name);
        let mut collector = ParseErrorCollector::new("test.hone".to_string());
        parse_test(&content, 1, &mut collector).is_some()
    }

    #[test]
    fn test_names_accept_any_characters() {
        assert!(parse_test_name("simple"));
        assert!(parse_test_name("Test123"));
        assert!(parse_test_name("my test name"));
        assert!(parse_test_name("test-with-dashes"));
        assert!(parse_test_name("test_with_underscores"));
        assert!(parse_test_name("test with equals = sign"));
        assert!(parse_test_name("test with 'single quotes'"));
        assert!(parse_test_name("test: with colon"));
        assert!(parse_test_name("is this valid?"));
        assert!(parse_test_name("test with @ symbol"));
        assert!(parse_test_name("test with # hash"));
        assert!(parse_test_name("test with $ dollar"));
        assert!(parse_test_name("test with * asterisk"));
        assert!(parse_test_name("test with | pipe"));
        assert!(parse_test_name("émojis 🎉 work too"));
    }

    #[test]
    fn test_empty_test_name_rejected() {
        let content = r#"TEST ""
RUN echo hello
ASSERT stdout contains "hello""#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Empty test name should produce an error"
                );
                assert!(
                    file.errors
                        .iter()
                        .any(|e| e.message.contains("cannot be empty")),
                    "Error message should mention empty test name"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_whitespace_only_test_name_allowed() {
        // Whitespace-only names should be allowed (for user flexibility)
        assert!(parse_test_name(" "));
        assert!(parse_test_name("   "));
    }

    #[test]
    fn test_error_node_in_ast() {
        let content = "TEST \"valid test\"\nINVALID LINE\nRUN echo hello";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                // Should have errors in the file
                assert!(!file.errors.is_empty());
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Unknown statement")));

                // Should also have error nodes in AST
                let error_nodes: Vec<_> = file
                    .nodes
                    .iter()
                    .filter(|n| matches!(n, ASTNode::Error(_)))
                    .collect();
                assert!(
                    !error_nodes.is_empty(),
                    "Expected error nodes in AST for invalid syntax"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_multiple_errors_collected() {
        let content = "INVALID1\nINVALID2\nTEST \"test\"\nINVALID3";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert_eq!(
                    file.errors.len(),
                    3,
                    "Expected 3 errors for 3 invalid lines"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_rejects_invalid_key_starting_with_number() {
        let content = "#!env: 123ABC=value";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for invalid env key"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid environment variable name")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_rejects_key_with_hyphen() {
        let content = "#!env: MY-VAR=value";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for invalid env key with hyphen"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid environment variable name")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_rejects_empty_key() {
        let content = "#!env: =value";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(!file.errors.is_empty(), "Expected error for empty env key");
                assert!(file.errors.iter().any(|e| e.message.contains("empty key")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_accepts_valid_key() {
        let content = "#!env: MY_VAR_123=value";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for valid env key"
                );
                let pragmas: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| match n {
                        ASTNode::Pragma(p) => Some(p),
                        _ => None,
                    })
                    .collect();
                assert_eq!(pragmas.len(), 1);
                assert_eq!(pragmas[0].key.as_ref().unwrap(), "MY_VAR_123");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_accepts_underscore_prefix() {
        let content = "#!env: _PRIVATE_VAR=value";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for underscore-prefixed env key"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_env_accepts_empty_value() {
        // Empty values are valid in shell (export FOO= sets FOO to empty string)
        let content = "#!env: MY_VAR=";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for empty env value"
                );
                let pragmas: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Pragma(p) = n {
                            Some(p)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(pragmas.len(), 1);
                assert_eq!(pragmas[0].value, "");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_accepts_empty_value() {
        // ENV statement with empty value should be valid
        let content = "ENV MY_VAR=\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for empty env value"
                );
                let envs: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Env(e) = n {
                            Some(e)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(envs.len(), 1);
                assert_eq!(envs[0].key, "MY_VAR");
                assert_eq!(envs[0].value, "");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_duplicate_run_names_rejected() {
        let content = r#"TEST "test with duplicates"
RUN build: echo first
RUN build: echo second
ASSERT stdout contains "first""#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for duplicate RUN names"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Duplicate RUN name")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_run_names_reset_between_tests() {
        let content = r#"TEST "first test"
RUN build: echo first
ASSERT stdout contains "first"

TEST "second test"
RUN build: echo second
ASSERT stdout contains "second""#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors - same RUN name in different tests should be allowed: {:?}",
                    file.errors
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_empty_run_command_rejected() {
        // "RUN " with just whitespace gets trimmed to "RUN" which is Unknown
        // This tests the lexer behavior
        let content = r#"TEST "empty run"
RUN 
ASSERT exit_code == 0"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for empty RUN command"
                );
                // "RUN" alone becomes Unknown statement since it doesn't match "RUN "
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Unknown statement")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_named_run_empty_command_rejected() {
        let content = r#"TEST "named empty run"
RUN build: 
ASSERT exit_code == 0"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for named RUN with empty command"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Empty command")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_exit_code_overflow_error_message() {
        let content = r#"TEST "overflow test"
RUN true
ASSERT exit_code == 9999999999999999999"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for overflowing exit code"
                );
                assert!(
                    file.errors.iter().any(|e| e.message.contains("too large")),
                    "Error message should mention value is too large"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_exit_code_rejects_relational_operators() {
        // exit_code only supports == and != operators, not <, <=, >, >=
        let operators = ["<", "<=", ">", ">="];
        for op in operators {
            let content = format!(
                r#"TEST "relational op test"
RUN true
ASSERT exit_code {} 0"#,
                op
            );
            let result = parse_file(&content, "test.hone");

            match result {
                ParseResult::Success { file } => {
                    assert!(
                        !file.errors.is_empty(),
                        "Expected error for exit_code with {} operator",
                        op
                    );
                    assert!(
                        file.errors.iter().any(|e| e.message.contains("== or !=")),
                        "Error message should indicate only == or != are allowed for operator {}",
                        op
                    );
                }
                ParseResult::Failure { .. } => {
                    panic!("Parser should always return Success with errors embedded");
                }
            }
        }
    }

    #[test]
    fn test_malformed_test_name_produces_error() {
        // Test with unclosed string
        let content = "TEST \"unclosed string";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for malformed TEST name"
                );
                assert!(
                    file.errors
                        .iter()
                        .any(|e| e.message.contains("Expected quoted string after TEST")),
                    "Error message should indicate string parsing failed"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_test_without_name_produces_error() {
        // Test with no string at all
        let content = "TEST ";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for TEST without name"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_pragma_missing_colon_produces_warning() {
        // Pragma without colon should produce a warning (e.g., "#!shell /bin/bash" instead of "#!shell: /bin/bash")
        let content = "#!shell /bin/bash\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.warnings.is_empty(),
                    "Expected warning for pragma missing colon"
                );
                assert!(file
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("Invalid pragma syntax")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_bare_pragma_produces_warning() {
        // Bare "#!" should produce a warning
        let content = "#!\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.warnings.is_empty(),
                    "Expected warning for bare pragma"
                );
                assert!(file
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("Invalid pragma syntax")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_missing_equals_produces_error() {
        // ENV without equals sign should produce an error
        let content = "ENV FOO\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for ENV missing equals sign"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid ENV syntax")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_rejects_invalid_key_starting_with_number() {
        let content = "ENV 123ABC=value\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for invalid env key starting with number"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid environment variable name")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_rejects_key_with_hyphen() {
        let content = "ENV MY-VAR=value\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for env key with hyphen"
                );
                assert!(file
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid environment variable name")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_rejects_empty_key() {
        let content = "ENV =value\nTEST \"test\"";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(!file.errors.is_empty(), "Expected error for empty env key");
                assert!(file.errors.iter().any(|e| e.message.contains("empty key")));
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_accepts_valid_key() {
        let content = "ENV MY_VAR_123=value\nTEST \"test\"\nRUN echo $MY_VAR_123";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for valid env key"
                );
                let envs: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Env(e) = n {
                            Some(e)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(envs.len(), 1);
                assert_eq!(envs[0].key, "MY_VAR_123");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_env_accepts_underscore_prefix() {
        let content = "ENV _PRIVATE=value\nTEST \"test\"\nRUN echo $_PRIVATE";
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.errors.is_empty(),
                    "Expected no errors for underscore-prefixed env key"
                );
                let envs: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Env(e) = n {
                            Some(e)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(envs.len(), 1);
                assert_eq!(envs[0].key, "_PRIVATE");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_parse_file_empty() {
        let result = parse_file("", "empty.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.nodes.is_empty(), "Empty file should have no nodes");
                assert!(file.pragmas.is_empty(), "Empty file should have no pragmas");
                assert!(file.errors.is_empty(), "Empty file should have no errors");
                assert!(
                    file.warnings.is_empty(),
                    "Empty file should have no warnings"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_parse_file_only_pragmas() {
        let content = "#!shell: /bin/bash\n#!timeout: 5s\n#!env: FOO=bar";
        let result = parse_file(content, "pragmas.hone");

        match result {
            ParseResult::Success { file } => {
                assert_eq!(file.pragmas.len(), 3, "Should have 3 pragmas");
                assert!(file.errors.is_empty(), "Should have no errors");
                // Pragmas are also added to nodes
                let pragma_nodes = file
                    .nodes
                    .iter()
                    .filter(|n| matches!(n, ASTNode::Pragma(_)))
                    .count();
                assert_eq!(pragma_nodes, 3, "Should have 3 pragma nodes");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_parse_file_only_comments() {
        let content = "# Comment 1\n# Comment 2\n# Comment 3";
        let result = parse_file(content, "comments.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.pragmas.is_empty(), "Should have no pragmas");
                assert!(file.errors.is_empty(), "Should have no errors");
                let comment_nodes = file
                    .nodes
                    .iter()
                    .filter(|n| matches!(n, ASTNode::Comment(_)))
                    .count();
                assert_eq!(comment_nodes, 3, "Should have 3 comment nodes");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_parse_file_only_whitespace() {
        let content = "   \n\n   \n\t\t\n";
        let result = parse_file(content, "whitespace.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    file.nodes.is_empty(),
                    "Whitespace-only file should have no nodes"
                );
                assert!(
                    file.errors.is_empty(),
                    "Whitespace-only file should have no errors"
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_file_predicate_error_mentions_matches() {
        // When an invalid file predicate is used, the error message should list ALL valid predicates
        let content = r#"TEST "file predicate test"
RUN echo hello > test.txt
ASSERT file "test.txt" invalid_predicate"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for invalid file predicate"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("matches"),
                    "Error message should mention 'matches' predicate. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_exit_code_missing_number_error() {
        // exit_code with operator but no number should produce a clear error
        let content = r#"TEST "missing number"
RUN true
ASSERT exit_code == "#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing number after exit_code operator"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("Expected number"),
                    "Error should mention expected number. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_exit_code_missing_operator_error() {
        // exit_code without operator should produce a clear error
        let content = r#"TEST "missing operator"
RUN true
ASSERT exit_code 0"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing operator in exit_code assertion"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("==") && error_msg.contains("!="),
                    "Error should mention valid operators (== or !=). Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_duration_missing_operator_error() {
        // duration without operator should produce a clear error
        let content = r#"TEST "missing operator"
RUN sleep 0.1
ASSERT duration 200ms"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing operator in duration assertion"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("comparison operator"),
                    "Error should mention comparison operator. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_duration_missing_value_error() {
        // duration with operator but no value should produce a clear error
        let content = r#"TEST "missing value"
RUN sleep 0.1
ASSERT duration < "#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing value after duration operator"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("duration value"),
                    "Error should mention expected duration value. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_output_contains_missing_string_error() {
        // stdout contains without string should produce a clear error
        let content = r#"TEST "missing string"
RUN echo hello
ASSERT stdout contains"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing string after 'contains'"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("quoted string") && error_msg.contains("contains"),
                    "Error should mention expected quoted string after contains. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_output_matches_missing_regex_error() {
        // stdout matches without regex should produce a clear error
        let content = r#"TEST "missing regex"
RUN echo hello
ASSERT stdout matches"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing regex after 'matches'"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("regex") && error_msg.contains("matches"),
                    "Error should mention expected regex literal after matches. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_output_equals_missing_string_error() {
        // stdout == without string should produce a clear error
        let content = r#"TEST "missing string"
RUN echo hello
ASSERT stdout == "#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing string after comparison operator"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("quoted string"),
                    "Error should mention expected quoted string. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_file_missing_path_error() {
        // file assertion without path should produce a clear error
        let content = r#"TEST "missing path"
RUN touch test.txt
ASSERT file exists"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing file path"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("quoted file path"),
                    "Error should mention expected quoted file path. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_file_contains_missing_string_error() {
        // file contains without string should produce a clear error
        let content = r#"TEST "missing string"
RUN echo hello > test.txt
ASSERT file "test.txt" contains"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing string after file contains"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("quoted string") && error_msg.contains("contains"),
                    "Error should mention expected quoted string after contains. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_file_matches_missing_regex_error() {
        // file matches without regex should produce a clear error
        let content = r#"TEST "missing regex"
RUN echo hello > test.txt
ASSERT file "test.txt" matches"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Expected error for missing regex after file matches"
                );
                let error_msg = &file.errors[0].message;
                assert!(
                    error_msg.contains("regex") && error_msg.contains("matches"),
                    "Error should mention expected regex literal. Got: {}",
                    error_msg
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_named_target_assertion_stdout() {
        let content = r#"TEST "named target"
RUN build: echo "hello"
ASSERT build.stdout contains "hello""#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Expected no errors");
                let assert_nodes: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Assert(a) = n {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(assert_nodes.len(), 1);

                if let AssertionExpression::Output {
                    target, selector, ..
                } = &assert_nodes[0].expression
                {
                    assert_eq!(target.as_deref(), Some("build"));
                    assert!(matches!(selector, OutputSelector::Stdout));
                } else {
                    panic!("Expected Output assertion expression");
                }
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_named_target_assertion_exit_code() {
        let content = r#"TEST "named target exit"
RUN build: echo "hello"
ASSERT build.exit_code == 0"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Expected no errors");
                let assert_nodes: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Assert(a) = n {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(assert_nodes.len(), 1);

                if let AssertionExpression::ExitCode { target, predicate } =
                    &assert_nodes[0].expression
                {
                    assert_eq!(target.as_deref(), Some("build"));
                    assert_eq!(predicate.value, 0);
                } else {
                    panic!("Expected ExitCode assertion expression");
                }
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_named_target_with_hyphen() {
        let content = r#"TEST "hyphenated target"
RUN my-build: echo "test"
ASSERT my-build.stdout contains "test""#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Expected no errors");
                let assert_nodes: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Assert(a) = n {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(assert_nodes.len(), 1);

                if let AssertionExpression::Output { target, .. } = &assert_nodes[0].expression {
                    assert_eq!(target.as_deref(), Some("my-build"));
                } else {
                    panic!("Expected Output assertion expression");
                }
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_named_target_duration() {
        let content = r#"TEST "duration target"
RUN fast: echo "quick"
ASSERT fast.duration < 100ms"#;
        let result = parse_file(content, "test.hone");

        match result {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Expected no errors");
                let assert_nodes: Vec<_> = file
                    .nodes
                    .iter()
                    .filter_map(|n| {
                        if let ASTNode::Assert(a) = n {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(assert_nodes.len(), 1);

                if let AssertionExpression::Duration { target, .. } = &assert_nodes[0].expression {
                    assert_eq!(target.as_deref(), Some("fast"));
                } else {
                    panic!("Expected Duration assertion expression");
                }
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_timeout_pragma_rejects_zero() {
        let input = r#"#! timeout: 0s
TEST "test"
RUN echo hello
"#;

        match parse_file(input, "test.hone") {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Should have parse error for 0s timeout"
                );
                assert!(
                    file.errors[0].message.contains("too small"),
                    "Error message should mention 'too small': {}",
                    file.errors[0].message
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_timeout_pragma_rejects_near_zero_ms() {
        let input = r#"#! timeout: 0.5ms
TEST "test"
RUN echo hello
"#;

        match parse_file(input, "test.hone") {
            ParseResult::Success { file } => {
                assert!(
                    !file.errors.is_empty(),
                    "Should have parse error for 0.5ms timeout"
                );
                assert!(
                    file.errors[0].message.contains("too small"),
                    "Error message should mention 'too small': {}",
                    file.errors[0].message
                );
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success with errors embedded");
            }
        }
    }

    #[test]
    fn test_timeout_pragma_accepts_1ms() {
        let input = r#"#! timeout: 1ms
TEST "test"
RUN echo hello
"#;

        match parse_file(input, "test.hone") {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Should accept 1ms timeout");
                assert_eq!(file.pragmas.len(), 1);
                assert_eq!(file.pragmas[0].value, "1ms");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }

    #[test]
    fn test_timeout_pragma_accepts_valid_seconds() {
        let input = r#"#! timeout: 30s
TEST "test"
RUN echo hello
"#;

        match parse_file(input, "test.hone") {
            ParseResult::Success { file } => {
                assert!(file.errors.is_empty(), "Should accept 30s timeout");
                assert_eq!(file.pragmas.len(), 1);
                assert_eq!(file.pragmas[0].value, "30s");
            }
            ParseResult::Failure { .. } => {
                panic!("Parser should always return Success");
            }
        }
    }
}
