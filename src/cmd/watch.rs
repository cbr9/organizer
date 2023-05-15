use std::{
	path::{Path, PathBuf},
	sync::mpsc::Sender,
	time::Duration,
};

use anyhow::Result;
use clap::Parser;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use organize_core::{config::Config, file::File};

use crate::{cmd::run::Run, Cmd};

#[derive(Parser, Debug)]
pub struct WatchBuilder {
	#[arg(long, short = 'c')]
	pub config: Option<PathBuf>,
	#[arg(long)]
	cleanup: Option<bool>,
	#[arg(long)]
	cleanup_after_reload: Option<bool>,
	#[arg(long)]
	delay: Option<u64>,
}

impl WatchBuilder {
	pub fn build(mut self) -> Result<Watch> {
		self.config = match self.config {
			Some(config) => Some(config),
			None => Some(Config::path()?),
		};
		self.cleanup = Some(self.cleanup.map_or_else(|| true, |v| !v));
		self.cleanup_after_reload = Some(self.cleanup_after_reload.map_or_else(|| true, |v| !v));
		self.delay = Some(self.delay.unwrap_or(0));

		Ok(Watch {
			config: Config::parse(self.config.unwrap())?,
			cleanup: self.cleanup.unwrap(),
			cleanup_after_reload: self.cleanup_after_reload.unwrap(),
			delay: Duration::new(self.delay.unwrap(), 0),
		})
	}
}

#[derive(Debug, Clone)]
pub struct Watch {
	pub config: Config,
	cleanup: bool,
	cleanup_after_reload: bool,
	delay: Duration,
}

impl Cmd for Watch {
	fn run(self) -> Result<()> {
		if self.cleanup {
			self.cleanup()?;
		}
		self.start()
	}
}

impl Watch {
	fn cleanup(&self) -> Result<()> {
		let cmd = Run { config: self.config.clone() };
		cmd.start()
	}

	fn on_create<T: AsRef<Path>>(&self, path: T) {
		let path = path.as_ref();
		let config_parent = self.config.path.parent().expect("Couldn't find config path");
		if let Some(parent) = path.parent() {
			if parent != config_parent && path.is_file() {
				let file = File::new(path, &self.config, true);
				file.act(&self.config.path_to_rules);
			}
		}
	}

	fn event_handler(
		&mut self,
		res: notify::Result<Event>,
		mut watcher: RecommendedWatcher,
		tx: &Sender<notify::Result<Event>>,
	) -> RecommendedWatcher {
		if let Ok(event) = res {
			match event.kind {
				notify::EventKind::Create(_) => {
					let copy = self.clone();
					std::thread::spawn(move || {
						std::thread::sleep(copy.delay);
						for path in event.paths {
							Self::on_create::<PathBuf>(&copy, path);
						}
					});
				}
				EventKind::Modify(_) => {
					for p in event.paths {
						if p == self.config.path {
							match Config::parse(&self.config.path) {
								Ok(new_config) => {
									self.config = new_config;
									log::info!("Reloaded config");
									watcher = self.setup(tx);
									if self.cleanup_after_reload {
										if let Err(e) = self.cleanup() {
											log::error!("{:?}", e);
										}
									}
								}
								Err(e) => log::error!("{:?}", e),
							}
						}
					}
				}
				_ => {}
			}
		}

		watcher
	}

	fn setup(&self, tx: &Sender<notify::Result<Event>>) -> RecommendedWatcher {
		let mut watcher = RecommendedWatcher::new(tx.clone(), notify::Config::default()).unwrap();

		for (folder, recursive) in self.config.path_to_recursive.iter() {
			watcher.watch(folder, recursive.type_()).unwrap();
		}

		if let Some(parent) = self.config.path.parent() {
			watcher.watch(parent, RecursiveMode::NonRecursive).unwrap();
		}
		watcher
	}

	fn start(mut self) -> Result<()> {
		let (tx, rx) = std::sync::mpsc::channel();
		let mut watcher = self.setup(&tx);

		for res in &rx {
			watcher = self.event_handler(res, watcher, &tx);
		}

		Ok(())
	}
}
