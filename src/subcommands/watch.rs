use std::{
    fs,
    io::Result,
    process,
    result,
    sync::mpsc::{channel, Receiver},
};

use colored::Colorize;
use log::{error, info};
use notify::{
    op,
    raw_watcher,
    RawEvent,
    RecommendedWatcher,
    RecursiveMode,
    Watcher as OtherWatcher,
};

use crate::{
    lock_file::GetProcessBy,
    path::{get_rules::GetRules, is_hidden::IsHidden},
    subcommands::run::run,
    user_config::{
        rules::{options::Options, rule::Rule},
        AsMap,
        PathToRules,
        UserConfig,
    },
    CONFIG,
    LOCK_FILE,
    MATCHES,
};
use std::{borrow::Borrow, collections::HashMap, ops::Deref, path::Path};
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

pub fn process_file(path: &Path, path2rules: &PathToRules, from_watch: bool) {
    if path.is_file() {
        let parent = path.parent().unwrap();
        'rules: for (rule, i) in path.get_rules(path2rules) {
            let folder = rule.folders.get(*i).unwrap();
            let Options {
                ignore,
                hidden_files,
                watch,
                apply,
                ..
            } = folder.options.as_ref().unwrap();
            if ignore.as_ref().unwrap().contains(&parent.to_path_buf()) {
                continue 'rules;
            }
            if path.is_hidden() && !hidden_files.unwrap() {
                continue 'rules;
            }
            if (!from_watch || watch.unwrap())
                && rule
                    .filters
                    .r#match(path, &apply.as_ref().unwrap().filters.as_ref().unwrap())
            {
                // simplified from `if (from_watch && *watch) || !from_watch`
                rule.actions
                    .run(&path, &apply.as_ref().unwrap().actions.as_ref().unwrap());
                break 'rules;
            }
        }
    }
}

pub fn watch() -> Result<()> {
    // REPLACE
    if MATCHES.subcommand().unwrap().1.is_present("replace") {
        Daemon::replace()?;
    } else {
        // FIXME: currently two instances can't be launched because we're not checking whether or not the new one has the same config as the running one
        let path = UserConfig::path();
        let watchers = LOCK_FILE.get_running_watchers();
        let mut running_configs = watchers.iter().map(|(_, path)| path);
        if running_configs.any(|config| config == &path) {
            return if path == UserConfig::default_path() {
                println!("An existing instance is already running. Use --replace to restart it");
                Ok(())
            } else {
                println!("An existing instance is already running with the selected configuration. Use --replace --config {} to restart it", path.display());
                Ok(())
            };
        }
        run()?;
        let mut watcher = Watcher::new();
        watcher.run(CONFIG.deref())?;
    }
    Ok(())
}

pub struct Watcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<RawEvent>,
}

impl Default for Watcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Watcher {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let watcher = raw_watcher(sender).unwrap();
        Watcher { watcher, receiver }
    }

    pub fn run<T>(&mut self, config: T) -> Result<()>
    where
        T: Borrow<UserConfig>,
    {
        let config = config.borrow();
        let mut folders = HashMap::new();
        for rule in config.rules.iter() {
            for folder in rule.folders.iter() {
                let recursive = if folder.options.as_ref().unwrap().recursive.unwrap() {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                let path = &folder.path;
                match folders.get(path) {
                    None => {
                        folders.insert(path, recursive);
                    }
                    Some(value) => {
                        if recursive == RecursiveMode::Recursive
                            && value == &RecursiveMode::NonRecursive
                        {
                            folders.insert(path, recursive);
                        }
                    }
                }
            }
        }

        if cfg!(feature = "hot-reload") {
            self.watcher
                .watch(config.path.parent().unwrap(), RecursiveMode::NonRecursive)
                .unwrap();
        }
        for (path, recursive) in folders.into_iter() {
            self.watcher.watch(path, recursive).unwrap();
        }

        // PROCESS SIGNALS
        LOCK_FILE.append(process::id() as i32, &config.path)?;
        let path2rules: HashMap<&Path, Vec<(&Rule, usize)>> = config.rules.map();

        loop {
            if let Ok(RawEvent {
                path: Some(path),
                op: Ok(op),
                ..
            }) = self.receiver.recv()
            {
                match op {
                    op::CREATE => {
                        if cfg!(not(feature = "hot-reload"))
                            || (cfg!(feature = "hot-reload")
                                && path.parent().unwrap() != config.path.parent().unwrap())
                        {
                            process_file(&path, &path2rules, true)
                        }
                    }
                    op::CLOSE_WRITE => {
                        if cfg!(feature = "hot-reload") && path == config.path {
                            let content = fs::read_to_string(&path).unwrap();
                            let new_config: result::Result<UserConfig, serde_yaml::Error> =
                                serde_yaml::from_str(&content);
                            match new_config {
                                Ok(mut new_config) => {
                                    new_config.path = config.path.clone();
                                    info!("reloaded configuration: {}", new_config.path.display());
                                    break self.run(new_config);
                                }
                                Err(e) => error!(
                                    "cannot reload config (rules will stay as they were): {}",
                                    e
                                ),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(crate) struct Daemon;

impl Daemon {
    pub fn replace() -> Result<()> {
        match LOCK_FILE.get_process_by(CONFIG.path.as_path()) {
            Some((pid, _)) => {
                let sys =
                    System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
                match sys.get_process(pid) {
                    None => {}
                    Some(process) => {
                        process.kill(Signal::Kill);
                    }
                }
                watch()
            }
            None => {
                // there is no running process
                if CONFIG.path == UserConfig::default_path() {
                    println!(
                        "{}",
                        "No instance was found running with the default configuration.".bold()
                    );
                } else {
                    println!(
                        "{} ({})",
                        "No instance was found running with the desired configuration".bold(),
                        CONFIG.path.display().to_string().underline()
                    );
                };
                Ok(())
            }
        }
    }
}
