use crate::parser::ast::*;
use crate::parser::errors::ParseErrorCollector;
use crate::parser::lexer::*;
use std::collections::HashSet;

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
                collector.add_error(format!("Unknown statement: {}", token.content), line_number);
            }
        }
    }

    if collector.has_errors() {
        ParseResult::Failure {
            errors: collector.get_errors(),
            warnings: collector.get_warnings(),
        }
    } else {
        ParseResult::Success {
            file: ParsedFile {
                filename: filename.to_string(),
                pragmas,
                nodes,
                warnings: collector.get_warnings(),
            },
        }
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

fn parse_test(content: &str, line: usize, collector: &mut ParseErrorCollector) -> Option<TestNode> {
    // TEST "name"
    let rest = &content[5..]; // After "TEST "
    let result = parse_string_literal(rest, 0)?;

    let name = result.0.value.clone();

    // Validate name characters
    let valid = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '_' || c == '-');

    if !valid {
        collector.add_error(
            format!(
                "Invalid test name: \"{}\". Names can only contain alphanumeric characters, spaces, dashes, and underscores",
                name
            ),
            line,
        );
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
    let re = regex::Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*):\s*").unwrap();
    if let Some(captures) = re.captures(rest) {
        let name = captures.get(1).unwrap().as_str().to_string();
        let matched_len = captures.get(0).unwrap().as_str().len();
        let command = rest[matched_len..].to_string();

        if run_names.contains(&name) {
            collector.add_error(
                format!(
                    "Duplicate RUN name: \"{}\". RUN names must be unique across the entire file",
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
    let re = regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
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
    let re = regex::Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)\.(.+)").unwrap();

    let mut effective_input = input;
    if let Some(captures) = re.captures(input) {
        let potential_target = captures.get(1).unwrap().as_str();
        if potential_target != "stdout"
            && potential_target != "stderr"
            && potential_target != "exit_code"
            && potential_target != "duration"
        {
            target = Some(potential_target.to_string());
            effective_input = captures.get(2).unwrap().as_str();
            i = 0;
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

        let str_result = parse_string_literal(input, i);
        if str_result.is_none() {
            collector.add_error(
                "Expected quoted string after \"contains\"".to_string(),
                line,
            );
            return None;
        }

        return Some(AssertionExpression::Output {
            target,
            selector,
            predicate: OutputPredicate::Contains {
                value: str_result.unwrap().0,
            },
        });
    }

    if match_word(input, i, "matches") {
        i += 7;
        i = skip_whitespace(input, i);

        let regex_result = parse_regex_literal(input, i);
        if regex_result.is_none() {
            collector.add_error("Expected regex literal after \"matches\"".to_string(), line);
            return None;
        }

        return Some(AssertionExpression::Output {
            target,
            selector,
            predicate: OutputPredicate::Matches {
                value: regex_result.unwrap().0,
            },
        });
    }

    // Check for == or !=
    if let Some((op, end_index)) = parse_comparison_operator(input, i) {
        if matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
            i = skip_whitespace(input, end_index);

            let str_result = parse_string_literal(input, i);
            if str_result.is_none() {
                collector.add_error(
                    "Expected quoted string after comparison operator".to_string(),
                    line,
                );
                return None;
            }

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
                    value: str_result.unwrap().0,
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
    let op_result = parse_comparison_operator(input, start_index);

    if op_result.is_none() {
        collector.add_error("Expected == or != after \"exit_code\"".to_string(), line);
        return None;
    }

    let (op, end_index) = op_result.unwrap();

    if !matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
        collector.add_error("Expected == or != after \"exit_code\"".to_string(), line);
        return None;
    }

    let num_result = parse_number(input, end_index);
    if num_result.is_none() {
        collector.add_error(
            "Expected number after comparison operator".to_string(),
            line,
        );
        return None;
    }

    let string_op = match op {
        ComparisonOperator::Equal => StringComparisonOperator::Equal,
        ComparisonOperator::NotEqual => StringComparisonOperator::NotEqual,
        _ => unreachable!(),
    };

    Some(AssertionExpression::ExitCode {
        target,
        predicate: ExitCodePredicate {
            operator: string_op,
            value: num_result.unwrap().0,
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
    let op_result = parse_comparison_operator(input, start_index);

    if op_result.is_none() {
        collector.add_error(
            "Expected comparison operator after \"duration\"".to_string(),
            line,
        );
        return None;
    }

    let (op, end_index) = op_result.unwrap();

    let duration_result = parse_duration(input, end_index);
    if duration_result.is_none() {
        collector.add_error(
            "Expected duration value (e.g., 200ms, 1.5s) after comparison operator".to_string(),
            line,
        );
        return None;
    }

    Some(AssertionExpression::Duration {
        target,
        predicate: DurationPredicate {
            operator: op,
            value: duration_result.unwrap().0,
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

        let str_result = parse_string_literal(input, i);
        if str_result.is_none() {
            collector.add_error(
                "Expected quoted string after \"contains\"".to_string(),
                line,
            );
            return None;
        }

        return Some(AssertionExpression::File {
            path,
            predicate: FilePredicate::Contains {
                value: str_result.unwrap().0,
            },
        });
    }

    if match_word(input, i, "matches") {
        i += 7;
        i = skip_whitespace(input, i);

        let regex_result = parse_regex_literal(input, i);
        if regex_result.is_none() {
            collector.add_error("Expected regex literal after \"matches\"".to_string(), line);
            return None;
        }

        return Some(AssertionExpression::File {
            path,
            predicate: FilePredicate::Matches {
                value: regex_result.unwrap().0,
            },
        });
    }

    // Check for == or !=
    if let Some((op, end_index)) = parse_comparison_operator(input, i) {
        if matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
            i = skip_whitespace(input, end_index);

            let str_result = parse_string_literal(input, i);
            if str_result.is_none() {
                collector.add_error(
                    "Expected quoted string after comparison operator".to_string(),
                    line,
                );
                return None;
            }

            let string_op = match op {
                ComparisonOperator::Equal => StringComparisonOperator::Equal,
                ComparisonOperator::NotEqual => StringComparisonOperator::NotEqual,
                _ => unreachable!(),
            };

            return Some(AssertionExpression::File {
                path,
                predicate: FilePredicate::Equals {
                    operator: string_op,
                    value: str_result.unwrap().0,
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
