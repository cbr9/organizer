mod lib;

use clap::crate_name;
use std::{
    env::temp_dir,
    fs,
    fs::OpenOptions,
    io::{prelude::*, Result},
    path::{Path, PathBuf},
};
use sysinfo::{Pid, RefreshKind, System, SystemExt};

/// File where watchers are registered with their PID and configuration
#[derive(Default)]
pub struct LockFile {
    path: PathBuf,
    sep: String,
}

pub trait GetProcessBy<T> {
    fn get_process_by(&self, val: T) -> Option<(Pid, PathBuf)>;
}

impl GetProcessBy<Pid> for LockFile {
    fn get_process_by(&self, val: Pid) -> Option<(Pid, PathBuf)> {
        self.get_running_watchers()
            .iter()
            .filter_map(|(pid, path)| {
                if *pid == val {
                    Some((*pid, path.clone()))
                } else {
                    None
                }
            })
            .next()
    }
}

impl<'a> GetProcessBy<&'a Path> for LockFile {
    fn get_process_by(&self, val: &'a Path) -> Option<(Pid, PathBuf)> {
        self.get_running_watchers()
            .iter()
            .filter_map(|(pid, path)| {
                if path == val {
                    Some((*pid, path.clone()))
                } else {
                    None
                }
            })
            .next()
    }
}

impl LockFile {
    pub fn new() -> Self {
        let lock_file = LockFile {
            path: temp_dir().join(format!("{}.lock", crate_name!())),
            sep: "---".into(),
        };
        lock_file
            .update()
            .expect("error: could not modify lock file (permission error?)");
        lock_file
    }

    fn section(&self, pid: &Pid, config: &Path) -> String {
        format!("{}\n{}\n{}", pid, config.display(), self.sep)
    }

    pub fn append(&self, pid: Pid, config: &Path) -> Result<()> {
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        let result = writeln!(f, "{}", self.section(&pid, config));
        result
    }

    pub fn get_running_watchers(&self) -> Vec<(Pid, PathBuf)> {
        let content = fs::read_to_string(&self.path);
        match content {
            Ok(content) => {
                if !content.is_empty() {
                    let content = content.trim().split(&self.sep);
                    content
                        .filter(|section| !section.is_empty() && *section != "\n")
                        .map(|section| {
                            let section = section
                                .lines()
                                .map(|line| line.to_string())
                                .filter(|line| !line.is_empty())
                                .collect::<Vec<_>>();
                            let pid = section.first().unwrap().parse().unwrap();
                            let path = section.get(1).unwrap().parse().unwrap();
                            (pid, path)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }

    fn update(&self) -> Result<()> {
        let mut running_processes = String::new();
        let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));

        for (pid, config) in self.get_running_watchers().iter() {
            let process = sys.get_process(*pid);
            if process.is_some() {
                running_processes.push_str(&self.section(pid, config));
                running_processes.push_str("\n");
            }
        }
        fs::write(&self.path, running_processes)?;
        Ok(())
    }
}
