use crate::parser::ast::ParseErrorDetail;
use owo_colors::OwoColorize;

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

#[derive(Debug, Clone)]
pub struct TestResults {
    pub total_files: usize,
    pub passed_files: usize,
    pub failed_files: usize,
    pub total_assertions: usize,
    pub passed_assertions: usize,
    pub failed_assertions: usize,
    pub failures: Vec<TestFailure>,
}

pub trait Reporter {
    fn on_file_start(&self, filename: &str);
    fn on_run_complete(&self, run_id: &str, success: bool);
    fn on_assertion_pass(&self);
    fn on_parse_errors(&self, errors: &[ParseErrorDetail]);
    fn on_warning(&self, message: &str);
    fn on_summary(&self, results: &TestResults);
    fn on_failure(&self, failure: &TestFailure);
}

pub struct DefaultReporter {
    verbose: bool,
}

impl DefaultReporter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl Reporter for DefaultReporter {
    fn on_file_start(&self, filename: &str) {
        println!("Running {}", filename);
    }

    fn on_run_complete(&self, run_id: &str, success: bool) {
        if success {
            print!("{}", "✓".green());
        } else {
            print!("{}", "✗".red());
        }
        if self.verbose && !run_id.is_empty() {
            print!(" ({})", run_id.dimmed());
        }
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }

    fn on_assertion_pass(&self) {
        if self.verbose {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    fn on_parse_errors(&self, errors: &[ParseErrorDetail]) {
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
        eprintln!("{} {}", "Warning:".yellow(), message);
    }

    fn on_summary(&self, results: &TestResults) {
        println!();

        if results.failed_files == 0 {
            let files_text = if results.total_files == 1 {
                "file"
            } else {
                "files"
            };
            let assertions_text = if results.passed_assertions == 1 {
                "assertion"
            } else {
                "assertions"
            };
            println!(
                "{} All tests passed ({} {}, {} {})",
                "✓".green(),
                results.total_files,
                files_text,
                results.passed_assertions,
                assertions_text
            );
        } else {
            let files_text = if results.total_files == 1 {
                "file"
            } else {
                "files"
            };
            println!(
                "{} {} of {} {} failed",
                "✗".red(),
                results.failed_files,
                results.total_files,
                files_text
            );
            println!(
                "  {} passed, {} failed",
                results.passed_assertions.dimmed(),
                results.failed_assertions.dimmed()
            );
        }
    }

    fn on_failure(&self, failure: &TestFailure) {
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
