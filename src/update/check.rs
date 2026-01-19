use relnotify::{ReleaseNotifier, ReleaseNotifierConfig};
use std::io::IsTerminal;
use std::path::PathBuf;

const REPO: &str = "captainsafia/hone";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_INTERVAL_HOURS: u64 = 24;

fn get_hone_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".hone"))
}

fn get_cache_path() -> Option<PathBuf> {
    get_hone_dir().map(|d| d.join("update-cache.json"))
}

fn is_update_check_disabled() -> bool {
    std::env::var("HONE_NO_UPDATE_CHECK")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

fn create_notifier() -> Option<ReleaseNotifier> {
    let hone_dir = get_hone_dir()?;
    let cache_path = get_cache_path()?;

    std::fs::create_dir_all(&hone_dir).ok()?;

    let config = ReleaseNotifierConfig::new(REPO)
        .check_interval(CHECK_INTERVAL_HOURS * 60 * 60)
        .cache_file_path(cache_path.to_string_lossy().to_string());

    ReleaseNotifier::new(config).ok()
}

pub fn spawn_update_check() {
    if is_update_check_disabled() || !is_tty() {
        return;
    }

    tokio::spawn(async {
        if let Some(notifier) = create_notifier() {
            let _ = notifier.check_version(CURRENT_VERSION, false).await;
        }
    });
}

pub fn show_update_notification_if_available() {
    if is_update_check_disabled() || !is_tty() {
        return;
    }

    let Some(notifier) = create_notifier() else {
        return;
    };

    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(notifier.check_version(CURRENT_VERSION, false))
    });

    let Ok(result) = result else {
        return;
    };

    if result.update_available {
        if let Some(release) = result.latest_release {
            let latest = release.tag_name.trim_start_matches('v');
            use owo_colors::OwoColorize;
            eprintln!(
                "\n{} {} → {}. Run {} to install.",
                "Update available:".dimmed(),
                format!("v{}", CURRENT_VERSION).dimmed(),
                format!("v{}", latest).green(),
                "`hone update`".cyan()
            );
        }
    }
}
