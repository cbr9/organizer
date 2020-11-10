use anyhow::Result;
use std::{
	process,
	sync::mpsc::{channel, Receiver},
};

use colored::Colorize;
use log::{error, info};
use notify::{op, raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{cmd::run::Run, Cmd, DEFAULT_CONFIG_STR};
use clap::Clap;
use lib::{
	config::{AsMap, Options, PathToRecursive, PathToRules, UserConfig},
	path::{GetRules, IsHidden},
	register::Register,
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

			let config = UserConfig::new(&self.config);
			Run { config: self.config.clone() }.start(&config)?;
			self.start(config)
		}
	}
}

impl Watch {
	fn replace(&self) -> Result<()> {
		let register = Register::new()?;
		match register.iter().find(|section| section.path == self.config) {
			Some(section) => {
				let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
				if let Some(process) = sys.get_process(section.pid) {
					process.kill(Signal::Kill);
				}
				self.start(UserConfig::new(&self.config))
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

	fn setup<T>(&self, config: T) -> Result<(RecommendedWatcher, Receiver<RawEvent>)>
	where
		T: AsRef<UserConfig>,
	{
		let mut folders: PathToRecursive = config.as_ref().rules.map();
		let (tx, rx) = channel();
		let mut watcher = raw_watcher(tx).unwrap();
		if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
			folders.insert(self.config.parent().unwrap(), RecursiveMode::NonRecursive);
		}
		for (folder, recursive) in folders.iter() {
			watcher.watch(folder, *recursive)?
		}
		Ok((watcher, rx))
	}

	fn start<T>(&self, config: T) -> Result<()>
	where
		T: AsRef<UserConfig>,
	{
		Register::new()?.append(process::id(), &self.config)?;
		let (mut watcher, rx) = self.setup(config.borrow())?;
		let path2rules = config.as_ref().rules.map();
		let config_parent = self.config.parent().unwrap();

		loop {
			match rx.recv() {
				#[rustfmt::skip]
				Ok(RawEvent { path: Some(path), op: Ok(op), .. }) => {
					match op {
						op::CREATE => {
							if let Some(parent) = path.parent() {
								if cfg!(not(feature = "hot-reload")) || (cfg!(feature = "hot-reload") && parent != config_parent) {
									process_file(&path, &path2rules, true)
								}
							}
						}
						op::CLOSE_WRITE => {
							if cfg!(feature = "hot-reload") && path == self.config {
								for folder in config.as_ref().rules.get_paths() {
									watcher.unwatch(folder)?;
								};
								watcher.unwatch(config_parent)?;
								std::mem::drop(path2rules);
								std::mem::drop(path);
                                std::mem::drop(config);
								let config = UserConfig::new(&self.config);
								info!("reloaded configuration: {}", self.config.display());
								break self.start(config);
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
			if (!from_watch || watch.unwrap()) && rule.filters.r#match(path, &apply.as_ref().unwrap().filters.as_ref().unwrap()) {
				// simplified from `if (from_watch && *watch) || !from_watch`
				rule.actions.run(&path, &apply.as_ref().unwrap().actions.as_ref().unwrap());
				break 'rules;
			}
		}
	}
}
