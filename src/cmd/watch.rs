use anyhow::Result;
use std::{
	process,
	sync::mpsc::{channel, Receiver},
};

use colored::Colorize;
use log::{debug, error, info};
use notify::{op, raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{cmd::run::Run, Cmd, DEFAULT_CONFIG_STR};
use clap::Clap;
use lib::{
	config::{ApplyWrapper, AsMap, Match, Options, UserConfig},
	file::File,
	register::Register,
	utils::UnwrapRef,
};
use std::{
	borrow::Borrow,
	path::{Path, PathBuf},
};
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

#[derive(Clap, Debug)]
pub struct Watch {
	#[clap(long, default_value = &DEFAULT_CONFIG_STR)]
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
				return if self.config == UserConfig::default_path() {
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
				Ok(config) => self.start(config),
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
					Ok(config) => self.start(config),
					Err(_) => std::process::exit(0),
				}
			}
			None => {
				// there is no running process
				if self.config == UserConfig::default_path() {
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

	fn setup(&self, config: &mut UserConfig) -> Result<(RecommendedWatcher, Receiver<RawEvent>)> {
		let folders = config.rules.path_to_recursive.as_mut().unwrap();
		let (tx, rx) = channel();
		let mut watcher = raw_watcher(tx).unwrap();
		if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
			folders.insert(self.config.parent().unwrap().to_path_buf(), RecursiveMode::NonRecursive);
		}
		for (folder, recursive) in folders.iter() {
			watcher.watch(folder, *recursive)?
		}
		Ok((watcher, rx))
	}

	fn start(&self, mut config: UserConfig) -> Result<()> {
		Register::new()?.append(process::id(), &self.config)?;
		let (mut watcher, rx) = self.setup(&mut config)?;
		let config_parent = self.config.parent().unwrap();

		loop {
			match rx.recv() {
				#[rustfmt::skip]
				Ok(RawEvent { path: Some(path), op: Ok(op), .. }) => {
					match op {
						op::CREATE => {
							if let Some(parent) = path.parent() {
								if (cfg!(not(feature = "hot-reload")) || (cfg!(feature = "hot-reload") && parent != config_parent)) && path.is_file() {
									let mut file = File::new(path);
									match config.defaults.unwrap_ref().r#match.unwrap_ref() {
										Match::All => file.get_matching_rules(config.as_ref()).into_iter().for_each(|(i, j)| {
											let rule = &config.rules[*i];
											rule.actions
												.run(&file.path, rule.folders[*j].options.unwrap_ref().apply.unwrap_ref().actions.unwrap_ref())
												.and_then(|f| {
													file.path = f;
													Ok(())
												});
										}),
										Match::First => {
											let (i, j) = file.get_matching_rules(config.as_ref()).into_iter().next().unwrap();
											let rule = &config.rules[*i];
											rule.actions
												.run(&file.path, rule.folders[*j].options.unwrap_ref().apply.unwrap_ref().actions.unwrap_ref())
												.and_then(|f| {
													file.path = f;
													Ok(())
												})?
										}
									}
								}
							}
						}
						op::CLOSE_WRITE => {
							if cfg!(feature = "hot-reload") && path == self.config {
								match UserConfig::new(&self.config) {
									Ok(new_config) => {
										for folder in config.rules.path_to_recursive.unwrap_ref().keys() {
											watcher.unwatch(folder)?;
										};
										watcher.unwatch(config_parent)?;
										std::mem::drop(path);
										std::mem::drop(config);
										info!("reloaded configuration: {}", self.config.display());
										break self.start(new_config);
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
