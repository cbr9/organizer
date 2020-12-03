use anyhow::Result;
use std::{
    process,
    sync::mpsc::{channel, Receiver},
};

use colored::Colorize;
use log::{debug, error, info};
use notify::{op, raw_watcher, watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher, DebouncedEvent};

use crate::{Cmd, CONFIG_PATH_STR};
use clap::Clap;
use organize_core::{
    data::{config::UserConfig, path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
    file::File,
    register::Register,
};
use std::path::PathBuf;
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct Watch {
    #[clap(long, default_value = &CONFIG_PATH_STR)]
    pub config: PathBuf,
    #[clap(long)]
    replace: bool,
}

impl Cmd for Watch {
    fn run(self) -> Result<()> {
        if self.replace {
            self.replace()
        } else {
            let register = Register::new()?;
            if register.iter().map(|section| &section.path).any(|config| config == &self.config) {
                return if self.config == UserConfig::path() {
                    println!("An existing instance is already running. Use --replace to restart it");
                    Ok(())
                } else {
                    println!(
                        "An existing instance is already running with the selected configuration. Use --replace --config {} to restart it",
                        self.config.display()
                    );
                    Ok(())
                };
            }

            match UserConfig::new(&self.config) {
                Ok(config) => {
                    let data = Data::from(config);
                    self.start(data)
                }
                Err(_) => std::process::exit(0),
            }
        }
    }
}

impl<'a> Watch {
    fn replace(&self) -> Result<()> {
        let register = Register::new()?;
        match register.iter().find(|section| section.path == self.config) {
            Some(section) => {
                let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
                if let Some(process) = sys.get_process(section.pid) {
                    process.kill(Signal::Kill);
                }
                match UserConfig::new(&self.config) {
                    // TODO: should check that it's valid before killing the previous process
                    Ok(config) => {
                        let data = Data::from(config);
                        self.start(data)
                    }
                    Err(_) => std::process::exit(0),
                }
            }
            None => {
                // there is no running process
                if self.config == UserConfig::path() {
                    println!("{}", "No instance was found running with the default configuration.".bold());
                } else {
                    println!(
                        "{} ({})",
                        "No instance was found running with the desired configuration".bold(),
                        self.config.display().to_string().underline()
                    );
                };
                Ok(())
            }
        }
    }

    fn setup(&'a self, data: &'a Data) -> Result<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
        let mut path_to_recursive = PathToRecursive::new(&data);
        if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
            path_to_recursive.insert(self.config.parent().unwrap(), RecursiveMode::NonRecursive);
        }
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
        for (folder, recursive) in path_to_recursive.iter() {
            watcher.watch(folder, *recursive)?
        }
        Ok((watcher, rx))
    }

    fn start(&'a self, data: Data) -> Result<()> {
        Register::new()?.append(process::id(), &self.config)?;
        let path_to_rules = PathToRules::new(&data);
        let (mut watcher, rx) = self.setup(&data)?;
        let config_parent = self.config.parent().unwrap();

        loop {
            match rx.recv() {
                #[rustfmt::skip]
                Ok(event) => {
                    match event {
                        DebouncedEvent::Create(path) => {
                            if let Some(parent) = path.parent() {
                                if (cfg!(not(feature = "hot-reload")) || (cfg!(feature = "hot-reload") && parent != config_parent)) && path.is_file() {
                                    let file = File::new(path);
                                    // std::thread::sleep(std::time::Duration::from_secs(1));
                                    file.process(&data, &path_to_rules);
                                }
                            }
                        }
                        DebouncedEvent::Write(path) => {
                            if cfg!(feature = "hot-reload") && path == self.config {
                                match UserConfig::new(&self.config) {
                                    Ok(new_config) =>{
                                        if new_config != data.config {
                                            for folder in path_to_rules.keys() {
                                                watcher.unwatch(folder)?;
                                            };
                                            if cfg!(feature = "hot-reload") {
                                                watcher.unwatch(self.config.parent().unwrap())?;
                                            }
                                            std::mem::drop(path);
                                            std::mem::drop(path_to_rules);
                                            std::mem::drop(data);
                                            let data = Data::from(new_config);
                                            info!("reloaded configuration: {}", self.config.display());
                                            break self.start(data);
                                        }
                                    }
                                    Err(_) => {
                                        debug!("could not reload configuration");
                                    }
                                };
                            }
                        }
                        _ => {}
                    }
                },
                Err(e) => error!("{}", e.to_string()),
                _ => {}
            }
        }
    }
}
