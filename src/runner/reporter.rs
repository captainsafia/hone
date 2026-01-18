use crate::parser::ast::ParseErrorDetail;
use owo_colors::OwoColorize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Passed,
    Failed,
    Skipped,
    Pending,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub file: String,
    pub shell: String,
    pub tests: Vec<TestResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub name: String,
    pub line: usize,
    pub status: Status,
    pub duration_ms: u64,
    pub runs: Vec<CommandRun>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandRun {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub command: String,
    pub line: usize,
    pub status: Status,
    pub duration_ms: u64,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub assertions: Vec<AssertionOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssertionOutput {
    pub line: usize,
    pub expression: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    #[serde(rename = "tests")]
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    #[serde(default)]
    pub pending: usize,
    #[serde(default)]
    pub skipped: usize,
    #[serde(default)]
    pub other: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub parse_errors: usize,
    #[serde(skip)]
    pub duration_ms: u64,
    #[serde(rename = "start")]
    pub start_time: u64,
    #[serde(rename = "stop")]
    pub stop_time: u64,
}

fn is_zero(val: &usize) -> bool {
    *val == 0
}

#[derive(Debug, Clone, Serialize)]
pub struct TestRunOutput {
    pub files: Vec<FileResult>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub report_format: &'static str,
    pub spec_version: &'static str,
    pub results: Results,
}

#[derive(Debug, Clone, Serialize)]
pub struct Results {
    pub tool: Tool,
    pub summary: Summary,
    pub tests: Vec<Test>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub name: String,
    pub status: Status,
    pub duration: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

impl TestRunOutput {
    pub fn has_failures(&self) -> bool {
        self.summary.failed > 0 || self.summary.parse_errors > 0
    }
}

pub trait OutputFormatter {
    fn format(&self, output: &TestRunOutput) -> String;
}

pub struct JsonFormatter;

impl JsonFormatter {
    fn build_failure_message(test: &TestResult) -> Option<String> {
        for run in &test.runs {
            for assertion in &run.assertions {
                if assertion.status == Status::Failed {
                    let mut parts = vec![assertion.expression.clone()];
                    if let Some(expected) = &assertion.expected {
                        parts.push(format!("Expected: {}", expected));
                    }
                    if let Some(actual) = &assertion.actual {
                        parts.push(format!("Actual: {}", actual));
                    }
                    return Some(parts.join("\n"));
                }
            }
        }
        None
    }

    fn build_trace(test: &TestResult, file: &FileResult) -> Option<String> {
        if test.status == Status::Failed {
            let mut trace_parts = vec![];
            for run in &test.runs {
                if run.status == Status::Failed {
                    trace_parts.push(format!(
                        "Command: {} (exit code: {})\nFile: {}:{}",
                        run.command, run.exit_code, file.file, run.line
                    ));
                    if !run.stderr.is_empty() {
                        trace_parts.push(format!("Stderr:\n{}", run.stderr));
                    }
                }
            }
            if !trace_parts.is_empty() {
                return Some(trace_parts.join("\n\n"));
            }
        }
        None
    }
}

impl OutputFormatter for JsonFormatter {
    fn format(&self, output: &TestRunOutput) -> String {
        let mut ctrf_tests = Vec::new();

        for file in &output.files {
            for test in &file.tests {
                let message = if test.status == Status::Failed {
                    Self::build_failure_message(test)
                } else {
                    None
                };

                let trace = Self::build_trace(test, file);

                ctrf_tests.push(Test {
                    name: test.name.clone(),
                    status: test.status,
                    duration: test.duration_ms,
                    file_path: Some(file.file.clone()),
                    line: Some(test.line),
                    message,
                    trace,
                });
            }
        }

        let report = Report {
            report_format: "CTRF",
            spec_version: "0.0.0",
            results: Results {
                tool: Tool {
                    name: "hone",
                    version: Some(env!("CARGO_PKG_VERSION")),
                },
                summary: output.summary.clone(),
                tests: ctrf_tests,
            },
        };

        serde_json::to_string_pretty(&report)
            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }
}

pub struct TextFormatter {
    pub verbose: bool,
}

impl OutputFormatter for TextFormatter {
    fn format(&self, output: &TestRunOutput) -> String {
        let mut result = String::new();

        if output.summary.parse_errors > 0 {
            let files_text = if output.summary.parse_errors == 1 {
                "file"
            } else {
                "files"
            };
            result.push_str(&format!(
                "{} {} {} had parse errors",
                "✗".red(),
                output.summary.parse_errors,
                files_text
            ));
        } else if output.summary.failed == 0 {
            let files_text = if output.files.len() == 1 {
                "file"
            } else {
                "files"
            };
            let total_assertions: usize = output
                .files
                .iter()
                .flat_map(|f| &f.tests)
                .flat_map(|t| &t.runs)
                .map(|r| r.assertions.len())
                .sum();
            let assertions_text = if total_assertions == 1 {
                "assertion"
            } else {
                "assertions"
            };
            result.push_str(&format!(
                "{} All tests passed ({} {}, {} {})",
                "✓".green(),
                output.files.len(),
                files_text,
                total_assertions,
                assertions_text
            ));
        } else {
            let files_text = if output.files.len() == 1 {
                "file"
            } else {
                "files"
            };
            let failed_files = output
                .files
                .iter()
                .filter(|f| f.tests.iter().any(|t| t.status == Status::Failed))
                .count();
            result.push_str(&format!(
                "{} {} of {} {} failed",
                "✗".red(),
                failed_files,
                output.files.len(),
                files_text
            ));
        }

        result
    }
}

// Legacy types for backward compatibility with streaming reporter
#[derive(Debug, Clone)]
pub struct TestFailure {
    pub filename: String,
    pub line: usize,
    pub test_name: Option<String>,
    pub run_command: Option<String>,
    pub assertion: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub error: Option<String>,
}

pub trait Reporter {
    fn on_file_start(&self, filename: &str);
    fn on_run_complete(&self, run_id: &str, success: bool);
    fn on_assertion_pass(&self);
    fn on_parse_errors(&self, errors: &[ParseErrorDetail]);
    fn on_warning(&self, message: &str);
    fn on_failure(&self, failure: &TestFailure);
}

pub struct DefaultReporter {
    verbose: bool,
    output_format: OutputFormat,
}

impl DefaultReporter {
    pub fn new(verbose: bool, output_format: OutputFormat) -> Self {
        Self {
            verbose,
            output_format,
        }
    }

    fn is_json(&self) -> bool {
        self.output_format == OutputFormat::Json
    }
}

impl Reporter for DefaultReporter {
    fn on_file_start(&self, filename: &str) {
        if !self.is_json() {
            println!("Running {}", filename);
        }
    }

    fn on_run_complete(&self, run_id: &str, success: bool) {
        if self.is_json() {
            return;
        }
        if success {
            print!("{}", "✓".green());
        } else {
            print!("{}", "✗".red());
        }
        if self.verbose && !run_id.is_empty() {
            print!(" ({})", run_id.dimmed());
        }
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    fn on_assertion_pass(&self) {
        if self.is_json() {
            return;
        }
        if self.verbose {
            print!(".");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    }

    fn on_parse_errors(&self, errors: &[ParseErrorDetail]) {
        if self.is_json() {
            return;
        }
        for error in errors {
            println!(
                "{} {} {}",
                "Parse Error:".red(),
                format!("{}:{}", error.filename, error.line).dimmed(),
                error.message
            );
        }
    }

    fn on_warning(&self, message: &str) {
        if self.is_json() {
            return;
        }
        eprintln!("{} {}", "Warning:".yellow(), message);
    }

    fn on_failure(&self, failure: &TestFailure) {
        if self.is_json() {
            return;
        }
        print_failure(failure, self.verbose);
    }
}

pub fn print_failure(failure: &TestFailure, verbose: bool) {
    println!();
    println!();

    let location_str = format!("{}:{}", failure.filename, failure.line);
    let location = location_str.dimmed();
    let test_name = failure
        .test_name
        .as_ref()
        .map(|name| format!(":: \"{}\"", name).dimmed().to_string())
        .unwrap_or_default();

    println!("{} {} {}", "FAIL".red(), location, test_name);

    if let Some(ref run_command) = failure.run_command {
        println!("{} {}", "RUN:".dimmed(), run_command);
    }

    if let Some(ref assertion) = failure.assertion {
        println!("{} {}", "ASSERT:".dimmed(), assertion);
    }

    if let Some(ref expected) = failure.expected {
        println!("{} {}", "Expected:".yellow(), expected);
    }

    if let Some(ref actual) = failure.actual {
        println!("{}", "Actual:".yellow());
        let lines: Vec<&str> = actual.split('\n').collect();
        let display_lines = if verbose {
            &lines[..]
        } else {
            &lines[..lines.len().min(10)]
        };

        for line in display_lines {
            println!("  {} {}", " ".dimmed(), line);
        }

        if !verbose && lines.len() > 10 {
            println!("  {} ... ({} more lines)", " ".dimmed(), lines.len() - 10);
        }
    }

    if let Some(ref error) = failure.error {
        println!("{} {}", "Error:".red(), error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(
        total_tests: usize,
        passed: usize,
        failed: usize,
        parse_errors: usize,
    ) -> Summary {
        Summary {
            total_tests,
            passed,
            failed,
            pending: 0,
            skipped: 0,
            other: 0,
            parse_errors,
            duration_ms: 0,
            start_time: 0,
            stop_time: 0,
        }
    }

    fn make_output(summary: Summary) -> TestRunOutput {
        TestRunOutput {
            files: vec![],
            summary,
        }
    }

    #[test]
    fn test_has_failures_all_passing() {
        let output = make_output(make_summary(5, 5, 0, 0));
        assert!(!output.has_failures());
    }

    #[test]
    fn test_has_failures_with_failed_tests() {
        let output = make_output(make_summary(5, 3, 2, 0));
        assert!(output.has_failures());
    }

    #[test]
    fn test_has_failures_with_parse_errors() {
        let output = make_output(make_summary(0, 0, 0, 1));
        assert!(output.has_failures());
    }

    #[test]
    fn test_has_failures_with_both_failures_and_parse_errors() {
        let output = make_output(make_summary(5, 3, 2, 1));
        assert!(output.has_failures());
    }

    #[test]
    fn test_has_failures_empty_run() {
        let output = make_output(make_summary(0, 0, 0, 0));
        assert!(!output.has_failures());
    }
}
