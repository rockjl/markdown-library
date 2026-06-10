use notify::{RecommendedWatcher, RecursiveMode, Event, Error, Watcher};
use std::sync::mpsc::{channel, Receiver};
use std::path::PathBuf;

pub struct FSWatcher {
    rx: Receiver<std::result::Result<Event, Error>>,
    _watcher: RecommendedWatcher,
}

impl FSWatcher {
    pub fn spawn(path: PathBuf) -> std::result::Result<Self, Error> {
        let (tx, rx) = channel();
        // RecommendedWatcher::new provides an implementation suitable for the platform
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

    pub fn try_recv(&self) -> Option<std::result::Result<Event, Error>> {
        match self.rx.try_recv() {
            Ok(ev) => Some(ev),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(_) => None,
        }
    }
}
