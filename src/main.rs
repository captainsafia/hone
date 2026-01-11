use clap::{Parser, Subcommand};
use hone::{run_tests, RunnerOptions};

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
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (patterns, shell, verbose) = match cli.command {
        Some(Commands::Run {
            patterns,
            shell,
            verbose,
        }) => (patterns, shell, verbose),
        None => (cli.patterns, cli.shell, cli.verbose),
    };

    let options = RunnerOptions { shell, verbose };

    let results = run_tests(patterns, options).await?;

    // Exit with code 1 if any tests failed
    std::process::exit(if results.failed_files > 0 { 1 } else { 0 });
}
