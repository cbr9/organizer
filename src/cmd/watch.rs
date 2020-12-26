use std::{
	process,
	sync::mpsc::{channel, Receiver},
};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Clap;
use colored::Colorize;
use log::{debug, error, info};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, watcher, Watcher};
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

use organize_core::{
	data::{config::Config, Data, path_to_recursive::PathToRecursive, path_to_rules::PathToRules},
	file::File,
	register::Register,
};
use organize_core::data::settings::Settings;
use organize_core::logger::Logger;

use crate::{Cmd, CONFIG_PATH_STR};
use crate::cmd::run::Run;

#[derive(Clap, Debug)]
pub struct Watch {
	#[clap(long, short = 'c', default_value = & CONFIG_PATH_STR, about = "Config path")]
	pub config: PathBuf,
	#[clap(long, short = 'd', default_value = "2", about = "Seconds to wait before processing an event")]
	delay: u8,
	#[clap(long, short = 'r', about = "Restart the instance running with the specified configuration")]
	replace: bool,
	#[clap(long, short = 's', about = "Do not change any files, but get output on the hypothetical changes")]
	simulate: bool,
	#[clap(long, about = "Process existing files before processing events")]
	clean: bool,
	#[clap(long, about = "Do not print colored output")]
	pub(crate) no_color: bool,
}

impl Cmd for Watch {
	fn run(mut self) -> Result<()> {
		Logger::setup(self.no_color)?;
		self.config = self.config.canonicalize()?;
		let data = Data::new()?;
		if self.clean {
			let cmd = Run {
				config: self.config.clone(),
				simulate: self.simulate,
				no_color: self.no_color,
			};
			cmd.start(data.clone())?;
		}
		if self.replace {
			self.replace()
		} else {
			let register = Register::new()?;
			if register.iter().map(|section| &section.path).any(|config| config == &self.config) {
				return if self.config == Config::default_path()? {
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
			self.start(data)
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
				self.start(Data::new()?)
			}
			None => {
				// there is no running process
				if self.config == Config::default_path()? {
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

	fn setup(&'a self, path_to_recursive: &PathToRecursive) -> Result<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
		let (tx, rx) = channel();
		let mut watcher = watcher(tx, Duration::from_secs(self.delay as u64)).unwrap();
		for (folder, (recursive, _)) in path_to_recursive.iter() {
			watcher.watch(folder, *recursive)?
		}
		if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
			watcher.watch(self.config.parent().unwrap(), RecursiveMode::NonRecursive)?;
		}
		Ok((watcher, rx))
	}

	fn start(&'a self, mut data: Data) -> Result<()> {
		Register::new()?.append(process::id(), &self.config)?;
		let path_to_rules = PathToRules::new(&data.config);
		let path_to_recursive = PathToRecursive::new(&data);
		let (mut watcher, rx) = self.setup(&path_to_recursive)?;
		let config_parent = self.config.parent().unwrap();

		loop {
			match rx.recv() {
				#[rustfmt::skip]
                Ok(event) => {
                    match event {
                        DebouncedEvent::Create(path) => {
                            if let Some(parent) = path.parent() {
                                if parent != config_parent && path.is_file() {
                                    let file = File::new(path);
                                    // std::thread::sleep(std::time::Duration::from_secs(1));
                                    file.process(&data, &path_to_rules, &path_to_recursive, self.simulate);
                                }
                            }
                        }
                        DebouncedEvent::Write(path) => {
                            if cfg!(feature = "hot-reload") {
                                if path == self.config {
                                    match Config::parse(&self.config) {
                                        Ok(new_config) => {
                                            if new_config != data.config {
                                                for folder in path_to_rules.keys() {
                                                    watcher.unwatch(folder)?;
                                                };
                                                if cfg!(feature = "hot-reload") {
                                                    watcher.unwatch(config_parent)?;
                                                }
                                                std::mem::drop(path);
                                                std::mem::drop(path_to_rules);
												std::mem::drop(path_to_recursive);
												data.config = new_config;
                                                info!("reloaded configuration: {}", self.config.display());
                                                break self.start(data);
                                            }
                                        }
                                        Err(e) => {
                                            debug!("could not reload configuration: {}", e);
                                        }
                                    };
                                } else if path == Settings::path() {
                                    match Settings::new(Settings::path()) {
                                        Ok(settings) => {
											if data.settings != settings {
												info!("successfully reloaded settings");
												data.settings = settings;
												break self.start(data);
											}
										}
                                        Err(e) => {
                                            debug!("could not reload settings: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
				Err(e) => error!("{}", e.to_string()),
			}
		}
	}
}
