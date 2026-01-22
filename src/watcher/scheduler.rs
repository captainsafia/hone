use std::collections::HashSet;
use std::path::PathBuf;

pub struct TestScheduler {
    running: bool,
    queued: HashSet<PathBuf>,
}

impl TestScheduler {
    pub fn new() -> Self {
        Self {
            running: false,
            queued: HashSet::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn start_run(&mut self) {
        self.running = true;
    }

    pub fn finish_run(&mut self) {
        self.running = false;
    }

    pub fn queue_files(&mut self, files: Vec<PathBuf>) {
        self.queued.extend(files);
    }

    pub fn take_queued(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.queued).into_iter().collect()
    }

    pub fn has_queued(&self) -> bool {
        !self.queued.is_empty()
    }
}

impl Default for TestScheduler {
    fn default() -> Self {
        Self::new()
    }
}
