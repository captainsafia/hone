use clap::{Parser, Subcommand};
use hone::{run_lsp_server, run_tests, OutputFormat, RunnerOptions};

mod setup;

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
    /// Start the Language Server Protocol (LSP) server
    Lsp,
    /// Setup editor integration for Hone
    #[command(long_about = "Setup editor integration for Hone

This command configures your editor to work with Hone files (.hone) by:
- Installing LSP (Language Server Protocol) configuration
- Adding syntax highlighting support
- Setting up file associations for .hone files

To remove the configuration manually:
- VS Code: Edit ~/.config/Code/User/settings.json (remove hone-related settings)
- Neovim: Edit ~/.config/nvim/init.lua or init.vim (remove hone-marked sections)
- Vim: Edit ~/.vimrc or ~/.vim/vimrc (remove hone-marked sections)
- Helix: Edit ~/.config/helix/languages.toml (remove [[language]] entry for hone)
- Emacs: Edit ~/.emacs.d/init.el or ~/.emacs (remove hone-marked sections)
- Sublime: Delete ~/.config/sublime-text/Packages/User/LSP.sublime-settings and Hone.sublime-syntax
- Zed: Edit ~/.config/zed/settings.json (remove hone-related settings)")]
    Setup {
        /// Editor(s) to configure (e.g., vscode, neovim, vim, helix, emacs, sublime, zed)
        editors: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Lsp) => {
            run_lsp_server().await?;
            Ok(())
        }
        Some(Commands::Setup { editors }) => {
            if editors.is_empty() {
                setup::list_editors();
                Ok(())
            } else {
                match setup::setup_editors(editors) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(2);
                    }
                }
            }
        }
        Some(Commands::Run {
            patterns,
            shell,
            verbose,
            test_filter,
            output_format,
        }) => {
            let options = RunnerOptions {
                shell,
                verbose,
                test_filter,
                output_format,
            };
            let results = run_tests(patterns, options).await?;
            std::process::exit(if results.has_failures() { 1 } else { 0 });
        }
        None => {
            let options = RunnerOptions {
                shell: cli.shell,
                verbose: cli.verbose,
                test_filter: cli.test_filter,
                output_format: cli.output_format,
            };
            let results = run_tests(cli.patterns, options).await?;
            std::process::exit(if results.has_failures() { 1 } else { 0 });
        }
    }
}
