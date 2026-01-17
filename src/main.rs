use clap::{Parser, Subcommand};
use hone::{run_tests, OutputFormat, RunnerOptions};

#[derive(Parser)]
#[command(name = "hone")]
#[command(about = "A CLI integration test runner for command-line applications")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Patterns to match test files (default subcommand)
    #[arg(value_name = "PATTERNS")]
    patterns: Vec<String>,

    /// Shell to use for tests
    #[arg(long)]
    shell: Option<String>,

    /// Enable verbose output
    #[arg(long, short)]
    verbose: bool,

    /// Filter tests by name (exact match or /regex/)
    #[arg(long = "test")]
    test_filter: Option<String>,

    /// Output format
    #[arg(long = "output-format", value_enum, default_value = "text")]
    output_format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests (default command)
    Run {
        /// Patterns to match test files
        patterns: Vec<String>,

        /// Shell to use for tests
        #[arg(long)]
        shell: Option<String>,

        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Filter tests by name (exact match or /regex/)
        #[arg(long = "test")]
        test_filter: Option<String>,

        /// Output format
        #[arg(long = "output-format", value_enum, default_value = "text")]
        output_format: OutputFormat,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (patterns, shell, verbose, test_filter, output_format) = match cli.command {
        Some(Commands::Run {
            patterns,
            shell,
            verbose,
            test_filter,
            output_format,
        }) => (patterns, shell, verbose, test_filter, output_format),
        None => (
            cli.patterns,
            cli.shell,
            cli.verbose,
            cli.test_filter,
            cli.output_format,
        ),
    };

    let options = RunnerOptions {
        shell,
        verbose,
        test_filter,
        output_format,
    };

    let results = run_tests(patterns, options).await?;

    // Exit with code 1 if any tests failed
    std::process::exit(if results.has_failures() { 1 } else { 0 });
}
