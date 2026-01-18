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
                if let Some(test) = parse_test(&token.content, line_number) {
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
    let colon_index = rest.find(':')?;

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
            let eq_index = pragma_value.find('=');
            if eq_index.is_none() {
                collector.add_error(format!("Invalid env pragma: {}", content), line);
                return None;
            }
            let eq_index = eq_index.unwrap();
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
            if parse_duration(pragma_value, 0).is_none() {
                collector.add_error(
                    format!(
                        "Invalid timeout format: {}. Expected format: <number>s or <number>ms",
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

fn parse_test(content: &str, line: usize) -> Option<TestNode> {
    // TEST "name"
    let rest = &content[5..]; // After "TEST "
    let result = parse_string_literal(rest, 0)?;

    let name = result.0.value.clone();
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
    let eq_index = rest.find('=')?;

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

    let Some((num_value, _)) = parse_number(input, end_index) else {
        collector.add_error(
            "Expected number after comparison operator".to_string(),
            line,
        );
        return None;
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

    let path_result = parse_string_literal(input, i);
    if path_result.is_none() {
        collector.add_error("Expected quoted file path after \"file\"".to_string(), line);
        return None;
    }

    let (path, end_index) = path_result.unwrap();
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
        "Expected predicate (exists, contains, ==, !=) after file path".to_string(),
        line,
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_name(name: &str) -> bool {
        let content = format!("TEST \"{}\"", name);
        parse_test(&content, 1).is_some()
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
}
