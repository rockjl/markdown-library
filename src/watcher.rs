//! Filesystem watcher for the content directory.

use notify::{RecommendedWatcher, RecursiveMode, Event, Error, Watcher};
use std::sync::mpsc::{channel, Receiver};
use std::path::PathBuf;

/// Wraps a platform-appropriate filesystem watcher for the content directory.
pub struct FSWatcher {
    /// Channel receiver for file-system events.
    rx: Receiver<std::result::Result<Event, Error>>,
    /// The underlying watcher handle (kept alive for the struct's lifetime).
    _watcher: RecommendedWatcher,
}

impl FSWatcher {
    /// Create a new filesystem watcher that monitors `path` recursively.
    ///
    /// Returns `Err` from the `notify` crate if the watcher cannot be initialised.
    pub fn spawn(path: PathBuf) -> std::result::Result<Self, Error> {
        let (tx, rx) = channel();
        let config = notify::Config::default();
        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, Error>| {
                let _ = tx.send(res);
            },
            config,
        )?;
        watcher.watch(&path, RecursiveMode::Recursive)?;
        Ok(Self { rx, _watcher: watcher })
    }

    /// Non-blocking read of the next filesystem event.
    ///
    /// Returns `None` if no event is pending.
    pub fn try_recv(&self) -> Option<std::result::Result<Event, Error>> {
        match self.rx.try_recv() {
            Ok(ev) => Some(ev),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(_) => None,
        }
    }
}
