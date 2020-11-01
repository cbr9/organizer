mod lib;

use clap::crate_name;
use std::{
    env::temp_dir,
    fs,
    fs::{File, OpenOptions},
    io::{prelude::*, Result},
    path::{Path, PathBuf},
};
use sysinfo::{Pid, RefreshKind, System, SystemExt};

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
        let f = LockFile {
            path: temp_dir().join(format!("{}.lock", crate_name!())),
            sep: "---".into(),
        };
        f.update()
            .expect("error: could not modify lock file (permission error?)");
        f
    }

    fn section(&self, pid: &Pid, config: &Path) -> String {
        format!("{}\n{}\n{}", pid, config.display(), self.sep)
    }

    fn set_readonly(&self, readonly: bool) -> Result<()> {
        let f = File::open(&self.path)?;
        let mut perms = f.metadata()?.permissions();
        perms.set_readonly(readonly);
        f.set_permissions(perms)?;
        Ok(())
    }

    pub fn append(&self, pid: Pid, config: &Path) -> Result<()> {
        if !self.path.exists() {
            File::create(&self.path)?;
        }
        self.set_readonly(false)?;
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        let result = writeln!(f, "{}", self.section(&pid, config));
        self.set_readonly(true)?;
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
        self.set_readonly(false)?;
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
        self.set_readonly(true)?;
        Ok(())
    }
}
