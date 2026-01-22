use crate::runner::{resolve_patterns, run_tests, RunnerOptions};
use crate::watcher::{FileWatcher, TestScheduler};
use owo_colors::OwoColorize;
use std::path::PathBuf;

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

pub async fn run_watch_mode(patterns: Vec<String>, options: RunnerOptions) -> anyhow::Result<()> {
    let files = resolve_patterns(&patterns).await?;
    if files.is_empty() {
        eprintln!("No .hone files found matching: {}", patterns.join(", "));
        return Ok(());
    }

    let file_paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let watcher = FileWatcher::new(file_paths, 500)?;
    let mut scheduler = TestScheduler::new();

    clear_screen();
    println!("{} Running initial tests...\n", "[watch]".cyan().bold());

    let _ = run_tests(files, options.clone()).await;

    println!(
        "\n{} Watching for changes. Press Ctrl+C to stop.\n",
        "[watch]".cyan().bold()
    );

    loop {
        if let Some(changed_files) = watcher.recv() {
            if scheduler.is_running() {
                scheduler.queue_files(changed_files);
            } else {
                scheduler.start_run();

                let mut files_to_run: Vec<PathBuf> = changed_files;

                loop {
                    clear_screen();
                    let file_strs: Vec<String> = files_to_run
                        .iter()
                        .map(|f| f.to_string_lossy().into_owned())
                        .collect();
                    println!(
                        "{} Re-running: {}\n",
                        "[watch]".cyan().bold(),
                        file_strs.join(", ")
                    );

                    let _ = run_tests(file_strs, options.clone()).await;

                    if scheduler.has_queued() {
                        files_to_run = scheduler.take_queued();
                    } else {
                        break;
                    }
                }

                scheduler.finish_run();
                println!(
                    "\n{} Watching for changes. Press Ctrl+C to stop.\n",
                    "[watch]".cyan().bold()
                );
            }
        }
    }
}
