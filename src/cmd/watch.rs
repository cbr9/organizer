use std::{
	path::{Path, PathBuf},
	process,
	sync::mpsc::{channel, Receiver},
	time::Duration,
};

#[cfg(feature = "interactive")]
use dialoguer::{theme::ColorfulTheme, Confirm};

use anyhow::Result;
use clap::Clap;
use colored::Colorize;
use log::{debug, info};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

use organize_core::{
	data::{config::Config, path_to_recursive::PathToRecursive, path_to_rules::PathToRules, settings::Settings, Data},
	file::File,
	logger::Logger,
	register::Register,
};

use crate::{cmd::run::Run, Cmd, CONFIG_PATH_STR};
use organize_core::simulation::Simulation;
use std::sync::{Arc, Mutex};

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
			self.cleanup(&data)?;
		}

		if self.replace {
			return self.replace();
		}

		let mut register = Register::new()?;

		if register.iter().any(|section| section.path == self.config) {
			println!("An existing instance is already running with the selected configuration. Add --replace to restart it");
			return Ok(());
		}
		register.push(process::id(), &self.config)?;
		self.start(data)
	}
}

impl<'a> Watch {
	fn cleanup(&self, data: &Data) -> Result<()> {
		let cmd = Run {
			config: self.config.clone(),
			simulate: self.simulate,
			no_color: self.no_color,
		};
		cmd.start(data.clone())
	}

	fn replace(&self) -> Result<()> {
		let register = Register::new()?;
		match register.iter().find(|section| section.path == self.config) {
			Some(section) => {
				let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
				if let Some(process) = sys.get_process(section.pid) {
					process.kill(Signal::Term);
				}
				self.start(Data::new()?)
			}
			None => self.replace_none(),
		}
	}

	#[cfg(feature = "interactive")]
	fn replace_none(&self) -> Result<()> {
		println!(
			"{} {}",
			"No instance was found running with configuration:".bold(),
			self.config.display().to_string().underline()
		);
		let prompt = Confirm::with_theme(&ColorfulTheme::default())
			.with_prompt("Do you wish to start an instance?")
			.interact()?;
		if prompt {
			return self.start(Data::new()?);
		}
		Ok(())
	}

	#[cfg(not(feature = "interactive"))]
	fn replace_none(&self) -> () {
		println!(
			"{} {}",
			"No instance was found running with configuration:".bold(),
			self.config.display().to_string().underline()
		);
		Ok(())
	}

	fn setup(&'a self, path_to_recursive: &PathToRecursive) -> Result<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
		let (tx, rx) = channel();
		let mut watcher = watcher(tx, Duration::from_secs(self.delay as u64)).unwrap();
		for (folder, recursive) in path_to_recursive.iter() {
			watcher.watch(folder, match recursive.is_recursive() {
				true => RecursiveMode::Recursive,
				false => RecursiveMode::NonRecursive,
			})?
		}
		if cfg!(feature = "hot-reload") && self.config.parent().is_some() {
			watcher.watch(self.config.parent().unwrap(), RecursiveMode::NonRecursive)?;
		}
		Ok((watcher, rx))
	}

	fn on_create<T: AsRef<Path>, P: AsRef<Path>>(
		path: T,
		config_parent: P,
		data: &Data,
		path_to_rules: &PathToRules,
		simulation: &Option<Arc<Mutex<Simulation>>>,
	) {
		let path = path.as_ref();
		let config_parent = config_parent.as_ref();
		if let Some(parent) = path.parent() {
			if parent != config_parent && path.is_file() {
				let file = File::new(path, data, true);
				match simulation {
					None => file.act(&path_to_rules),
					Some(simulation) => file.simulate(&path_to_rules, simulation),
				}
			}
		}
	}

	fn start(&'a self, mut data: Data) -> Result<()> {
		let path_to_rules = PathToRules::new(&data.config);
		let path_to_recursive = PathToRecursive::new(&data);
		let (mut watcher, rx) = self.setup(&path_to_recursive)?;
		let config_parent = self.config.parent().unwrap();
		let settings_path = Settings::path()?;
		let simulation = if self.simulate { Some(Simulation::new()?) } else { None };

		loop {
			if let Ok(event) = rx.recv() {
				match event {
					DebouncedEvent::Create(path) => Self::on_create(path, config_parent, &data, &path_to_rules, &simulation),
					DebouncedEvent::Write(path) => {
						if cfg!(feature = "hot-reload") {
							if path == self.config {
								match Config::parse(&self.config) {
									Ok(new_config) => {
										if new_config != data.config {
											for folder in path_to_rules.keys() {
												watcher.unwatch(folder)?;
											}
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
							} else if path == settings_path {
								match Settings::new(&settings_path) {
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
		}
	}
}
