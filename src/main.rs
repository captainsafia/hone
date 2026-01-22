use clap::{Parser, Subcommand};
use hone::{run_lsp_server, run_tests, run_watch_mode, OutputFormat, RunnerOptions};

mod setup;
mod update;

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

    /// Watch mode: re-run tests when files change
    #[arg(long, short)]
    watch: bool,
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

        /// Watch mode: re-run tests when files change
        #[arg(long, short)]
        watch: bool,
    },
    /// Start the Language Server Protocol (LSP) server
    Lsp,
    /// Setup editor integration for Hone
    #[command(long_about = "Setup editor integration for Hone

This command configures your editor to work with Hone files (.hone) by:
- Installing LSP (Language Server Protocol) configuration
- Adding syntax highlighting support
- Setting up file associations for .hone files

To remove the configuration manually:
- VS Code: Uninstall the hone extension via the Extensions panel
- Neovim: Edit ~/.config/nvim/init.lua or init.vim (remove hone-marked sections)
- Vim: Edit ~/.vimrc or ~/.vim/vimrc (remove hone-marked sections)")]
    Setup {
        /// Editor(s) to configure (e.g., vscode, neovim, vim)
        editors: Vec<String>,
    },
    /// Update Hone to a newer version
    Update {
        /// Target version to install (default: latest)
        #[arg(value_name = "VERSION")]
        version: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let skip_update_check = matches!(
        cli.command,
        Some(Commands::Lsp) | Some(Commands::Update { .. })
    );

    if !skip_update_check {
        update::spawn_update_check();
    }

    match cli.command {
        Some(Commands::Lsp) => {
            run_lsp_server().await?;
            Ok(())
        }
        Some(Commands::Setup { editors }) => {
            if editors.is_empty() {
                setup::list_editors();
            } else {
                match setup::setup_editors(editors) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(2);
                    }
                }
            }
            update::show_update_notification_if_available();
            Ok(())
        }
        Some(Commands::Update { version }) => {
            update::perform_update(version).await?;
            Ok(())
        }
        Some(Commands::Run {
            patterns,
            shell,
            verbose,
            test_filter,
            output_format,
            watch,
        }) => {
            let options = RunnerOptions {
                shell,
                verbose,
                test_filter,
                output_format,
            };
            if watch {
                run_watch_mode(patterns, options).await?;
                Ok(())
            } else {
                let results = run_tests(patterns, options).await?;
                update::show_update_notification_if_available();
                std::process::exit(if results.has_failures() { 1 } else { 0 });
            }
        }
        None => {
            let options = RunnerOptions {
                shell: cli.shell,
                verbose: cli.verbose,
                test_filter: cli.test_filter,
                output_format: cli.output_format,
            };
            if cli.watch {
                run_watch_mode(cli.patterns, options).await?;
                Ok(())
            } else {
                let results = run_tests(cli.patterns, options).await?;
                update::show_update_notification_if_available();
                std::process::exit(if results.has_failures() { 1 } else { 0 });
            }
        }
    }
}
