use std::collections::HashSet;
use std::path::{PathBuf, Path};

use std::sync::mpsc::{channel, Receiver};
use notify::{Watcher, RawEvent, Op, RecommendedWatcher, RecursiveMode};

struct SimFiles {
    files: HashSet<PathBuf>,
    folders: HashSet<PathBuf>,
    receiver: Receiver<RawEvent>,
    watcher: RecommendedWatcher,
}

impl SimFiles {
    fn new() -> Result<Self, notify::Error> {
        let (sender, receiver) = channel();
        let watcher = notify::raw_watcher(sender)?;

        let sim = Self {
            files: HashSet::new(),
            folders: HashSet::new(),
            receiver,
            watcher,
        };

        Ok(sim)
    }

    fn watch_folder<T: Into<PathBuf>>(&mut self, folder: T) -> anyhow::Result<()> {
        let path = folder.into();
        let files = path.read_dir()?.filter_map(|file| Some(file.ok()?.path()));
        self.files.extend(files);
        self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
        self.folders.insert(path);
        Ok(())
    }

    fn unwatch_folder<T: AsRef<Path>>(&mut self, folder: T) -> Result<(), notify::Error> {
        let folder = folder.as_ref();
        let folders = &self.folders;
        self.files.retain(|file| {
            if let Some(parent) = file.parent() {
                !folders.contains(parent)
            } else {
                false
            }
        });
        self.folders.remove(folder);
        self.watcher.unwatch(folder)
    }

    fn insert_file<T: Into<PathBuf>>(&mut self, file: T) -> bool {
        self.files.insert(file.into())
    }

    fn remove_file<T: AsRef<Path>>(&mut self, file: T) -> bool {
        self.files.remove(file.as_ref())
    }

    fn sync(&mut self) {
        loop {
            match self.receiver.recv() {
                Ok(RawEvent { path: Some(path), op: Ok(op), .. }) => {
                    match op {
                        Op::REMOVE => {
                            self.remove_file(path);
                        },
                        Op::CREATE => {
                            self.insert_file(path);
                        },
                        _ => {},
                    };
                },
                Err(_) => {},
                _ => {}
            }
        }
    }
}