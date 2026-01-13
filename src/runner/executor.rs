use crate::assertions::{
    exitcode::evaluate_exit_code_predicate,
    filesystem::evaluate_file_predicate,
    output::{evaluate_output_predicate, get_output_value},
    timing::evaluate_duration_predicate,
    AssertionResult,
};
use crate::parse_file;
use crate::parser::ast::{ASTNode, AssertNode, ParseResult};
use crate::runner::reporter::{DefaultReporter, Reporter, TestFailure, TestResults};
use crate::runner::shell::{create_shell_config, RunResult, ShellSession};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct RunnerOptions {
    pub shell: Option<String>,
    pub verbose: bool,
}

struct FileRunResult {
    passed: bool,
    assertions_passed: usize,
    assertions_failed: usize,
    failure: Option<TestFailure>,
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
}

pub async fn run_tests(
    patterns: Vec<String>,
    options: RunnerOptions,
) -> anyhow::Result<TestResults> {
    let reporter = DefaultReporter::new(options.verbose);
    let cwd = std::env::current_dir()?.to_string_lossy().to_string();

    let mut all_files = BTreeSet::new();
    for pattern in &patterns {
        let files = resolve_files(pattern, &cwd).await?;
        all_files.extend(files);
    }
    let all_files: Vec<_> = all_files.into_iter().collect();

    if all_files.is_empty() {
        reporter.on_warning(&format!(
            "No test files found matching: {}",
            patterns.join(", ")
        ));
        return Ok(TestResults {
            total_files: 0,
            passed_files: 0,
            failed_files: 0,
            total_assertions: 0,
            passed_assertions: 0,
            failed_assertions: 0,
            failures: vec![],
        });
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
    let mut parse_failures = Vec::new();
    let mut valid_files = Vec::new();

    for result in parse_results {
        match result {
            Ok((file, parse_result)) => {
                match parse_result {
                    ParseResult::Failure { errors, warnings } => {
                        reporter.on_parse_errors(&errors);
                        for error in &errors {
                            parse_failures.push(TestFailure {
                                filename: error.filename.clone(),
                                line: error.line,
                                test_name: None,
                                run_command: None,
                                assertion: None,
                                expected: None,
                                actual: None,
                                error: Some(error.message.clone()),
                            });
                        }
                        // Report warnings too
                        for warning in &warnings {
                            reporter.on_warning(&format!(
                                "{}:{} :: {}",
                                warning.filename, warning.line, warning.message
                            ));
                        }
                    }
                    ParseResult::Success { file: parsed_file } => {
                        // Report warnings
                        for warning in &parsed_file.warnings {
                            reporter.on_warning(&format!(
                                "{}:{} :: {}",
                                warning.filename, warning.line, warning.message
                            ));
                        }
                        valid_files.push((file, parsed_file.nodes));
                    }
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    // Run each file sequentially
    let mut results = Vec::new();

    for (file, ast) in valid_files {
        let result = run_file(&ast, &file, &options, &reporter).await?;
        results.push(result);
    }

    // Compile final results
    let total_assertions = results
        .iter()
        .map(|r| r.assertions_passed + r.assertions_failed)
        .sum();
    let passed_assertions: usize = results.iter().map(|r| r.assertions_passed).sum();
    let failed_assertions: usize = results.iter().map(|r| r.assertions_failed).sum();
    let mut failures = parse_failures;
    failures.extend(results.iter().filter_map(|r| r.failure.clone()));

    let test_results = TestResults {
        total_files: all_files.len(),
        passed_files: results.iter().filter(|r| r.passed).count(),
        failed_files: all_files.len() - results.iter().filter(|r| r.passed).count(),
        total_assertions,
        passed_assertions,
        failed_assertions,
        failures,
    };

    reporter.on_summary(&test_results);

    Ok(test_results)
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
    reporter: &impl Reporter,
) -> anyhow::Result<FileRunResult> {
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
    let test_blocks = group_nodes_by_test(ast);

    let mut total_assertions_passed = 0;
    let mut failure: Option<TestFailure> = None;

    for block in test_blocks {
        // Create a fresh shell session for each TEST block
        let mut session = ShellSession::new(shell_config.clone());

        match session.start().await {
            Ok(_) => {}
            Err(e) => {
                failure = Some(TestFailure {
                    filename: filename.to_string(),
                    line: block.test_node.as_ref().map(|n| n.line()).unwrap_or(0),
                    test_name: block.test_name.clone(),
                    run_command: None,
                    assertion: None,
                    expected: None,
                    actual: None,
                    error: Some(format!("Failed to start shell: {}", e)),
                });
                break;
            }
        }

        let result = execute_test_block(&block, &mut session, filename, reporter).await;
        let _ = session.stop().await;

        total_assertions_passed += result.assertions_passed;

        if let Some(f) = result.failure {
            failure = Some(f);
            break;
        }
    }

    println!(); // Newline after progress dots

    if let Some(ref f) = failure {
        reporter.on_failure(f);
        return Ok(FileRunResult {
            passed: false,
            assertions_passed: total_assertions_passed,
            assertions_failed: 1,
            failure,
        });
    }

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

    Ok(FileRunResult {
        passed: true,
        assertions_passed: total_assertions_passed,
        assertions_failed: 0,
        failure: None,
    })
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
    if let Some(ref test_name) = block.test_name {
        session.set_current_test(Some(test_name.clone()));
    }

    let mut last_run_result: Option<RunResult> = None;
    let mut run_results: HashMap<String, RunResult> = HashMap::new();
    let mut assertions_passed = 0;
    let mut pending_env_vars: Vec<(String, String)> = Vec::new();

    for node in &block.nodes {
        match node {
            ASTNode::Env(env_node) => {
                pending_env_vars.push((env_node.key.clone(), env_node.value.clone()));
            }

            ASTNode::Run(run_node) => {
                // Apply any pending env vars before the run
                if !pending_env_vars.is_empty() {
                    if let Err(e) = session.set_env_vars(&pending_env_vars).await {
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
                    }
                    Err(e) => {
                        reporter.on_run_complete("", false);
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

                if !result.passed {
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
                    };
                }

                assertions_passed += 1;
                reporter.on_assertion_pass();
            }

            _ => {}
        }
    }

    ExecuteResult {
        assertions_passed,
        failure: None,
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
            let shell_cwd = session.get_cwd().await.unwrap_or_else(|_| ".".to_string());
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
