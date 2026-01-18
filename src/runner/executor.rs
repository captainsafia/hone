use crate::assertions::{
    exitcode::evaluate_exit_code_predicate,
    filesystem::evaluate_file_predicate,
    output::{evaluate_output_predicate, get_output_value},
    timing::evaluate_duration_predicate,
    AssertionResult,
};
use crate::parse_file;
use crate::parser::ast::{ASTNode, AssertNode, ParseResult, RunNode};
use crate::runner::reporter::{
    AssertionOutput, CommandRun, DefaultReporter, FileResult, JsonFormatter, OutputFormat,
    OutputFormatter, Reporter, Status, Summary, TestFailure, TestResult, TestRunOutput,
    TextFormatter,
};
use crate::runner::shell::{create_shell_config, RunResult, ShellSession};
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct RunnerOptions {
    pub shell: Option<String>,
    pub verbose: bool,
    pub test_filter: Option<String>,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub enum TestFilter {
    Exact(String),
    Regex(Regex),
}

impl TryFrom<&str> for TestFilter {
    type Error = String;

    fn try_from(pattern: &str) -> Result<Self, Self::Error> {
        if pattern.starts_with('/') && pattern.ends_with('/') && pattern.len() > 2 {
            let regex_pattern = &pattern[1..pattern.len() - 1];
            Regex::new(regex_pattern)
                .map(TestFilter::Regex)
                .map_err(|e| format!("Invalid regex pattern: {}", e))
        } else {
            Ok(TestFilter::Exact(pattern.to_string()))
        }
    }
}

impl TestFilter {
    pub fn matches(&self, test_name: &str) -> bool {
        match self {
            TestFilter::Exact(pattern) => test_name == pattern,
            TestFilter::Regex(regex) => regex.is_match(test_name),
        }
    }
}

struct FileRunResult {
    file_result: FileResult,
}

#[derive(Default)]
struct TestBlock {
    test_name: Option<String>,
    test_node: Option<ASTNode>,
    nodes: Vec<ASTNode>,
}

struct ExecuteResult {
    assertions_passed: usize,
    failure: Option<TestFailure>,
    test_result: Option<TestResult>,
}

pub async fn run_tests(
    patterns: Vec<String>,
    options: RunnerOptions,
) -> anyhow::Result<TestRunOutput> {
    let is_json = options.output_format == OutputFormat::Json;
    let reporter = DefaultReporter::new(options.verbose, options.output_format);
    let cwd = std::env::current_dir()?.to_string_lossy().to_string();
    let start_time = std::time::Instant::now();
    let start_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Validate test filter early if provided
    let test_filter = if let Some(ref filter_pattern) = options.test_filter {
        match TestFilter::try_from(filter_pattern.as_str()) {
            Ok(filter) => Some(filter),
            Err(e) => {
                return Err(anyhow::anyhow!("Invalid test filter: {}", e));
            }
        }
    } else {
        None
    };

    let mut all_files = BTreeSet::new();
    for pattern in &patterns {
        let files = resolve_files(pattern, &cwd).await?;
        all_files.extend(files);
    }
    let all_files: Vec<_> = all_files.into_iter().collect();

    if all_files.is_empty() {
        if !is_json {
            reporter.on_warning(&format!(
                "No test files found matching: {}",
                patterns.join(", ")
            ));
        }
        let stop_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let output = TestRunOutput {
            files: vec![],
            summary: Summary {
                total_tests: 0,
                passed: 0,
                failed: 0,
                pending: 0,
                skipped: 0,
                other: 0,
                parse_errors: 0,
                duration_ms: start_time.elapsed().as_millis() as u64,
                start_time: start_epoch,
                stop_time: stop_epoch,
            },
        };
        match options.output_format {
            OutputFormat::Json => {
                let formatter = JsonFormatter;
                println!("{}", formatter.format(&output));
            }
            OutputFormat::Text => {}
        }
        return Ok(output);
    }

    // Parse all files first (can be done in parallel)
    let parse_futures: Vec<_> = all_files
        .iter()
        .map(|file| async move {
            let content = tokio::fs::read_to_string(file).await?;
            let result = parse_file(&content, file);
            Ok::<(String, ParseResult), anyhow::Error>((file.clone(), result))
        })
        .collect();

    let parse_results = futures::future::join_all(parse_futures).await;

    // Collect parse errors and valid files
    let mut valid_files = Vec::new();
    let mut parse_error_count = 0;

    for result in parse_results {
        match result {
            Ok((file, parse_result)) => match parse_result {
                ParseResult::Success { file: parsed_file } => {
                    // Report errors if any
                    if !parsed_file.errors.is_empty() {
                        if !is_json {
                            reporter.on_parse_errors(&parsed_file.errors);
                        }
                        parse_error_count += 1;
                        // Skip files with errors in CLI mode
                        continue;
                    }

                    // Report warnings
                    if !is_json {
                        for warning in &parsed_file.warnings {
                            reporter.on_warning(&format!(
                                "{}:{} :: {}",
                                warning.filename, warning.line, warning.message
                            ));
                        }
                    }
                    valid_files.push((file, parsed_file.nodes));
                }
                ParseResult::Failure { errors, warnings } => {
                    // Legacy path - should not be reached with new parser
                    if !is_json {
                        reporter.on_parse_errors(&errors);
                        for warning in &warnings {
                            reporter.on_warning(&format!(
                                "{}:{} :: {}",
                                warning.filename, warning.line, warning.message
                            ));
                        }
                    }
                    parse_error_count += 1;
                }
            },
            Err(e) => {
                return Err(e);
            }
        }
    }

    // Run each file sequentially
    let mut file_results = Vec::new();

    for (file, ast) in valid_files {
        let result = run_file(&ast, &file, &options, test_filter.as_ref(), &reporter).await?;
        file_results.push(result.file_result);
    }

    // Build output
    let total_tests: usize = file_results.iter().map(|f| f.tests.len()).sum();
    let passed_tests: usize = file_results
        .iter()
        .flat_map(|f| &f.tests)
        .filter(|t| t.status == Status::Passed)
        .count();
    let failed_tests = total_tests - passed_tests;
    let stop_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let output = TestRunOutput {
        files: file_results,
        summary: Summary {
            total_tests,
            passed: passed_tests,
            failed: failed_tests,
            pending: 0,
            skipped: 0,
            other: 0,
            parse_errors: parse_error_count,
            duration_ms: start_time.elapsed().as_millis() as u64,
            start_time: start_epoch,
            stop_time: stop_epoch,
        },
    };

    // Format and print output
    match options.output_format {
        OutputFormat::Json => {
            let formatter = JsonFormatter;
            println!("{}", formatter.format(&output));
        }
        OutputFormat::Text => {
            let formatter = TextFormatter {
                verbose: options.verbose,
            };
            println!();
            println!("{}", formatter.format(&output));
        }
    }

    Ok(output)
}

async fn resolve_files(pattern: &str, cwd: &str) -> anyhow::Result<Vec<String>> {
    let resolved = PathBuf::from(cwd).join(pattern);

    // Check if it's a direct file
    if let Ok(metadata) = tokio::fs::metadata(&resolved).await {
        if metadata.is_file() && pattern.ends_with(".hone") {
            return Ok(vec![resolved.to_string_lossy().to_string()]);
        }

        if metadata.is_dir() {
            // Use glob to find all .hone files in the directory
            let pattern = format!("{}/**/*.hone", resolved.to_string_lossy());
            let paths = glob::glob(&pattern)?;
            let results: Vec<String> = paths
                .filter_map(Result::ok)
                .filter(|p| p.is_file())
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            return Ok(results);
        }
    }

    // Use glob for pattern matching
    let glob_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_string()
    } else {
        PathBuf::from(cwd)
            .join(pattern)
            .to_string_lossy()
            .to_string()
    };

    let paths = glob::glob(&glob_pattern)?;
    let results: Vec<String> = paths
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    Ok(results)
}

async fn run_file(
    ast: &[ASTNode],
    filename: &str,
    options: &RunnerOptions,
    test_filter: Option<&TestFilter>,
    reporter: &impl Reporter,
) -> anyhow::Result<FileRunResult> {
    let is_json = options.output_format == OutputFormat::Json;
    let cwd = Path::new(filename)
        .parent()
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .to_string();

    let basename = Path::new(filename)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    reporter.on_file_start(&basename);

    // Extract pragmas
    let pragmas: Vec<_> = ast
        .iter()
        .filter_map(|node| {
            if let ASTNode::Pragma(pragma) = node {
                Some(pragma.clone())
            } else {
                None
            }
        })
        .collect();

    // Create shell config from pragmas
    let shell_config = create_shell_config(&pragmas, filename, &cwd, options.shell.as_deref());

    // Group nodes by TEST block
    let mut test_blocks = group_nodes_by_test(ast);

    // Apply test filter if provided
    if let Some(filter) = test_filter {
        test_blocks.retain(|block| {
            block
                .test_name
                .as_ref()
                .map(|name| filter.matches(name))
                .unwrap_or(false)
        });
    }

    let mut total_assertions_passed = 0;
    let mut failure: Option<TestFailure> = None;
    let mut test_results: Vec<TestResult> = Vec::new();

    for block in test_blocks {
        let test_start = std::time::Instant::now();
        let test_line = block.test_node.as_ref().map(|n| n.line()).unwrap_or(1);
        let test_name = block.test_name.clone().unwrap_or_default();

        // Create a fresh shell session for each TEST block
        let mut session = ShellSession::new(shell_config.clone());

        match session.start().await {
            Ok(_) => {}
            Err(e) => {
                failure = Some(TestFailure {
                    filename: filename.to_string(),
                    line: test_line,
                    test_name: block.test_name.clone(),
                    run_command: None,
                    assertion: None,
                    expected: None,
                    actual: None,
                    error: Some(format!("Failed to start shell: {}", e)),
                });

                test_results.push(TestResult {
                    name: test_name,
                    line: test_line,
                    status: Status::Failed,
                    duration_ms: test_start.elapsed().as_millis() as u64,
                    runs: vec![],
                });
                break;
            }
        }

        let result = execute_test_block(&block, &mut session, filename, reporter).await;
        let _ = session.stop().await;

        total_assertions_passed += result.assertions_passed;

        if let Some(test_result) = result.test_result {
            test_results.push(test_result);
        }

        if let Some(f) = result.failure {
            failure = Some(f);
            break;
        }
    }

    if !is_json {
        println!(); // Newline after progress dots
    }

    let file_result = FileResult {
        file: filename.to_string(),
        shell: shell_config.shell.clone(),
        tests: test_results,
    };

    if let Some(ref f) = failure {
        reporter.on_failure(f);
    } else if !is_json {
        let assertions_text = if total_assertions_passed == 1 {
            "assertion"
        } else {
            "assertions"
        };

        use owo_colors::OwoColorize;
        println!(
            "PASS {} ({} {})",
            basename,
            total_assertions_passed,
            assertions_text.green()
        );
    }

    Ok(FileRunResult { file_result })
}

fn group_nodes_by_test(nodes: &[ASTNode]) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut current_block = TestBlock::default();

    for node in nodes {
        match node {
            ASTNode::Test(test_node) => {
                if current_block.test_name.is_some() || !current_block.nodes.is_empty() {
                    blocks.push(std::mem::take(&mut current_block));
                }
                current_block.test_name = Some(test_node.name.clone());
                current_block.test_node = Some(node.clone());
            }
            ASTNode::Pragma(_) | ASTNode::Comment(_) => {}
            _ => {
                current_block.nodes.push(node.clone());
            }
        }
    }

    if current_block.test_name.is_some() || !current_block.nodes.is_empty() {
        blocks.push(current_block);
    }

    blocks
}

async fn execute_test_block(
    block: &TestBlock,
    session: &mut ShellSession,
    filename: &str,
    reporter: &impl Reporter,
) -> ExecuteResult {
    let test_start = std::time::Instant::now();

    if let Some(ref test_name) = block.test_name {
        session.set_current_test(Some(test_name.clone()));
    }

    let mut last_run_result: Option<RunResult> = None;
    let mut last_run_node: Option<&RunNode> = None;
    let mut run_results: HashMap<String, RunResult> = HashMap::new();
    let mut assertions_passed = 0;
    let mut pending_env_vars: Vec<(String, String)> = Vec::new();

    // Track runs with their assertions
    let mut command_runs: Vec<CommandRun> = Vec::new();
    let mut current_run_assertions: Vec<AssertionOutput> = Vec::new();

    for node in &block.nodes {
        match node {
            ASTNode::Env(env_node) => {
                pending_env_vars.push((env_node.key.clone(), env_node.value.clone()));
            }

            ASTNode::Run(run_node) => {
                // Finalize previous run if any
                if let (Some(prev_result), Some(prev_node)) = (&last_run_result, last_run_node) {
                    let run_status = if current_run_assertions
                        .iter()
                        .all(|a| a.status == Status::Passed)
                    {
                        Status::Passed
                    } else {
                        Status::Failed
                    };
                    command_runs.push(CommandRun {
                        name: prev_node.name.clone(),
                        command: prev_node.command.clone(),
                        line: prev_node.line,
                        status: run_status,
                        duration_ms: prev_result.duration_ms,
                        exit_code: prev_result.exit_code,
                        stdout: prev_result.stdout.clone(),
                        stderr: prev_result.stderr.clone(),
                        assertions: std::mem::take(&mut current_run_assertions),
                    });
                }

                // Apply any pending env vars before the run
                if !pending_env_vars.is_empty() {
                    if let Err(e) = session.set_env_vars(&pending_env_vars).await {
                        let test_result = TestResult {
                            name: block.test_name.clone().unwrap_or_default(),
                            line: block.test_node.as_ref().map(|n| n.line()).unwrap_or(1),
                            status: Status::Failed,
                            duration_ms: test_start.elapsed().as_millis() as u64,
                            runs: command_runs,
                        };
                        return ExecuteResult {
                            assertions_passed,
                            failure: Some(TestFailure {
                                filename: filename.to_string(),
                                line: node.line(),
                                test_name: block.test_name.clone(),
                                run_command: None,
                                assertion: None,
                                expected: None,
                                actual: None,
                                error: Some(format!("Failed to set environment variables: {}", e)),
                            }),
                            test_result: Some(test_result),
                        };
                    }
                    pending_env_vars.clear();
                }

                match session
                    .run(&run_node.command, run_node.name.as_deref())
                    .await
                {
                    Ok(result) => {
                        reporter.on_run_complete(&result.run_id, true);
                        if let Some(ref name) = run_node.name {
                            run_results.insert(name.clone(), result.clone());
                        }
                        last_run_result = Some(result);
                        last_run_node = Some(run_node);
                    }
                    Err(e) => {
                        reporter.on_run_complete("", false);

                        // Add the failed run
                        command_runs.push(CommandRun {
                            name: run_node.name.clone(),
                            command: run_node.command.clone(),
                            line: run_node.line,
                            status: Status::Failed,
                            duration_ms: 0,
                            exit_code: -1,
                            stdout: String::new(),
                            stderr: e.clone(),
                            assertions: vec![],
                        });

                        let test_result = TestResult {
                            name: block.test_name.clone().unwrap_or_default(),
                            line: block.test_node.as_ref().map(|n| n.line()).unwrap_or(1),
                            status: Status::Failed,
                            duration_ms: test_start.elapsed().as_millis() as u64,
                            runs: command_runs,
                        };

                        return ExecuteResult {
                            assertions_passed,
                            failure: Some(TestFailure {
                                filename: filename.to_string(),
                                line: node.line(),
                                test_name: block.test_name.clone(),
                                run_command: Some(run_node.command.clone()),
                                assertion: None,
                                expected: None,
                                actual: None,
                                error: Some(e),
                            }),
                            test_result: Some(test_result),
                        };
                    }
                }
            }

            ASTNode::Assert(assert_node) => {
                let result = evaluate_assertion(
                    assert_node,
                    last_run_result.as_ref(),
                    &run_results,
                    session,
                )
                .await;

                let assertion_output = AssertionOutput {
                    line: assert_node.line,
                    expression: assert_node.raw.clone(),
                    status: if result.passed {
                        Status::Passed
                    } else {
                        Status::Failed
                    },
                    expected: if result.passed {
                        None
                    } else {
                        Some(result.expected.clone())
                    },
                    actual: if result.passed {
                        None
                    } else {
                        Some(result.actual.clone())
                    },
                };
                current_run_assertions.push(assertion_output);

                if !result.passed {
                    // Finalize the current run
                    if let (Some(prev_result), Some(prev_node)) = (&last_run_result, last_run_node)
                    {
                        command_runs.push(CommandRun {
                            name: prev_node.name.clone(),
                            command: prev_node.command.clone(),
                            line: prev_node.line,
                            status: Status::Failed,
                            duration_ms: prev_result.duration_ms,
                            exit_code: prev_result.exit_code,
                            stdout: prev_result.stdout.clone(),
                            stderr: prev_result.stderr.clone(),
                            assertions: std::mem::take(&mut current_run_assertions),
                        });
                    }

                    let test_result = TestResult {
                        name: block.test_name.clone().unwrap_or_default(),
                        line: block.test_node.as_ref().map(|n| n.line()).unwrap_or(1),
                        status: Status::Failed,
                        duration_ms: test_start.elapsed().as_millis() as u64,
                        runs: command_runs,
                    };

                    return ExecuteResult {
                        assertions_passed,
                        failure: Some(TestFailure {
                            filename: filename.to_string(),
                            line: node.line(),
                            test_name: block.test_name.clone(),
                            run_command: last_run_result.as_ref().map(|r| r.run_id.clone()),
                            assertion: Some(assert_node.raw.clone()),
                            expected: Some(result.expected),
                            actual: Some(result.actual),
                            error: result.error,
                        }),
                        test_result: Some(test_result),
                    };
                }

                assertions_passed += 1;
                reporter.on_assertion_pass();
            }

            _ => {}
        }
    }

    // Finalize the last run
    if let (Some(prev_result), Some(prev_node)) = (&last_run_result, last_run_node) {
        let run_status = if current_run_assertions
            .iter()
            .all(|a| a.status == Status::Passed)
        {
            Status::Passed
        } else {
            Status::Failed
        };
        command_runs.push(CommandRun {
            name: prev_node.name.clone(),
            command: prev_node.command.clone(),
            line: prev_node.line,
            status: run_status,
            duration_ms: prev_result.duration_ms,
            exit_code: prev_result.exit_code,
            stdout: prev_result.stdout.clone(),
            stderr: prev_result.stderr.clone(),
            assertions: current_run_assertions,
        });
    }

    let test_result = TestResult {
        name: block.test_name.clone().unwrap_or_default(),
        line: block.test_node.as_ref().map(|n| n.line()).unwrap_or(1),
        status: Status::Passed,
        duration_ms: test_start.elapsed().as_millis() as u64,
        runs: command_runs,
    };

    ExecuteResult {
        assertions_passed,
        failure: None,
        test_result: Some(test_result),
    }
}

async fn evaluate_assertion(
    node: &AssertNode,
    last_run_result: Option<&RunResult>,
    run_results: &HashMap<String, RunResult>,
    session: &mut ShellSession,
) -> AssertionResult {
    let expr = &node.expression;

    match expr {
        crate::parser::ast::AssertionExpression::Output {
            target,
            selector,
            predicate,
        } => {
            let target_result = match resolve_target(target, last_run_result, run_results) {
                Ok(result) => result,
                Err(assertion) => return assertion,
            };

            let output = get_output_value(target_result, selector);
            evaluate_output_predicate(output, predicate)
        }

        crate::parser::ast::AssertionExpression::ExitCode { target, predicate } => {
            let target_result = match resolve_target(target, last_run_result, run_results) {
                Ok(result) => result,
                Err(assertion) => return assertion,
            };

            evaluate_exit_code_predicate(target_result.exit_code, predicate)
        }

        crate::parser::ast::AssertionExpression::Duration { target, predicate } => {
            let target_result = match resolve_target(target, last_run_result, run_results) {
                Ok(result) => result,
                Err(assertion) => return assertion,
            };

            evaluate_duration_predicate(target_result.duration_ms, predicate)
        }

        crate::parser::ast::AssertionExpression::File { path, predicate } => {
            let shell_cwd = match session.get_cwd().await {
                Ok(cwd) => cwd,
                Err(e) => {
                    return AssertionResult::with_error(
                        false,
                        format!("file \"{}\" check", path.value),
                        "failed to get current working directory".to_string(),
                        format!("Error: {}", e),
                    )
                }
            };
            evaluate_file_predicate(path, predicate, &shell_cwd).await
        }
    }
}

fn resolve_target<'a>(
    target: &Option<String>,
    last_run_result: Option<&'a RunResult>,
    run_results: &'a HashMap<String, RunResult>,
) -> Result<&'a RunResult, AssertionResult> {
    if let Some(ref target_name) = target {
        run_results.get(target_name).ok_or_else(|| {
            AssertionResult::new(
                false,
                format!("RUN named \"{}\" to exist", target_name),
                "RUN not found".to_string(),
            )
        })
    } else {
        last_run_result.ok_or_else(|| {
            AssertionResult::with_error(
                false,
                "a previous RUN command".to_string(),
                "no RUN command executed".to_string(),
                "ASSERT without a preceding RUN".to_string(),
            )
        })
    }
}
