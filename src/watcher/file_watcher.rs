use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
    receiver: Receiver<Vec<PathBuf>>,
}

impl FileWatcher {
    pub fn new(files: Vec<PathBuf>, debounce_ms: u64) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let mut debouncer = new_debouncer(
            Duration::from_millis(debounce_ms),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    let changed_files: HashSet<PathBuf> = events
                        .into_iter()
                        .filter(|e| {
                            matches!(
                                e.kind,
                                EventKind::Create(_)
                                    | EventKind::Modify(ModifyKind::Data(_))
                                    | EventKind::Remove(_)
                            )
                        })
                        .flat_map(|e| e.paths.clone())
                        .filter(|p| p.extension().is_some_and(|ext| ext == "hone"))
                        .collect();

                    if !changed_files.is_empty() {
                        let _ = tx.send(changed_files.into_iter().collect());
                    }
                }
            },
        )?;

        for file in &files {
            debouncer
                .watch(file, RecursiveMode::NonRecursive)
                .map_err(|e| anyhow::anyhow!("Failed to watch {}: {}", file.display(), e))?;
        }

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
        })
    }

    pub fn recv(&self) -> Option<Vec<PathBuf>> {
        self.receiver.recv().ok()
    }
}
