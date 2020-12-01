use anyhow::Result;
use std::{
	process,
	sync::mpsc::{channel, Receiver},
};

use colored::Colorize;
use log::{debug, error, info};
use notify::{op, raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{Cmd, CONFIG_PATH_STR};
use clap::Clap;
use organize_core::{
	config::UserConfig,
	data::{Data, PathToRecursive, PathToRules},
	file::File,
	register::Register,
	utils::UnwrapRef,
};
use std::path::PathBuf;
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

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
					self.start(data, PathToRules::new(&data), PathToRecursive::new(&data))
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
						self.start(data, PathToRules::new(&data), PathToRecursive::new(&data))
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

	fn setup(&'a self, mut path_to_recursive: &'a mut PathToRecursive<'a>) -> Result<(RecommendedWatcher, Receiver<RawEvent>)> {
		let (tx, rx) = channel();
		let mut watcher = raw_watcher(tx).unwrap();
		if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
			path_to_recursive.insert(self.config.parent().unwrap(), RecursiveMode::NonRecursive);
		}
		for (folder, recursive) in path_to_recursive.iter() {
			watcher.watch(folder, *recursive)?
		}
		Ok((watcher, rx))
	}

	fn start(&'a self, mut data: Data, path_to_rules: PathToRules<'a>, mut path_to_recursive: PathToRecursive<'a>) -> Result<()> {
		Register::new()?.append(process::id(), &self.config)?;
		let (mut watcher, rx) = self.setup(&mut path_to_recursive)?;
		let config_parent = self.config.parent().unwrap();

		loop {
			match rx.recv() {
				#[rustfmt::skip]
				Ok(RawEvent { path: Some(path), op: Ok(op), .. }) => {
					match op {
						op::CREATE => {
							if let Some(parent) = path.parent() {
								if (cfg!(not(feature = "hot-reload")) || (cfg!(feature = "hot-reload") && parent != config_parent)) && path.is_file() {
									let file = File::new(path);
									file.process(&data, &path_to_rules);
								}
							}
						}
						op::CLOSE_WRITE => {
							if cfg!(feature = "hot-reload") && path == self.config {
								match UserConfig::new(&self.config) {
									Ok(new_config) => {
										for folder in path_to_recursive.keys() {
											watcher.unwatch(folder)?;
										};
										watcher.unwatch(config_parent)?;
										std::mem::drop(path);
										std::mem::drop(data);
										std::mem::drop(path_to_recursive);
										std::mem::drop(path_to_rules);
										let data = Data::from(new_config);
										let path_to_rules = PathToRules::new(&data);
										let path_to_recursive = PathToRecursive::new(&data);
										info!("reloaded configuration: {}", self.config.display());
										break self.start(data, path_to_rules, path_to_recursive);
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
