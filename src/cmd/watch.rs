use std::{
	path::{Path, PathBuf},
	sync::mpsc::Sender,
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
	clean: Option<bool>,
	#[arg(long)]
	pub(crate) no_color: Option<bool>,
}

impl WatchBuilder {
	pub fn build(mut self) -> Result<Watch> {
		self.config = match self.config {
			Some(config) => Some(config),
			None => Some(Config::path()?),
		};
		self.no_color = Some(self.no_color.map_or_else(|| true, |v| !v));
		self.clean = Some(self.clean.map_or_else(|| true, |v| !v));

		Ok(Watch {
			config: Config::parse(unsafe { self.config.unwrap_unchecked() }).unwrap(),
			clean: unsafe { self.clean.unwrap_unchecked() },
			no_color: unsafe { self.no_color.unwrap_unchecked() },
		})
	}
}

#[derive(Debug)]
pub struct Watch {
	pub config: Config,
	clean: bool,
	pub(crate) no_color: bool,
}

impl Cmd for Watch {
	fn run(self) -> Result<()> {
		// Logger::setup(self.no_color)?;
		if self.clean {
			self.cleanup()?;
		}
		self.start()
	}
}

impl Watch {
	fn cleanup(&self) -> Result<()> {
		let cmd = Run::builder()
			.config(Some(self.config.path.clone()))?
			.no_color(Some(self.no_color))
			.build()?;
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
		let event = res.unwrap();
		match event.kind {
			notify::EventKind::Create(_) => {
				for path in event.paths {
					self.on_create::<PathBuf>(path);
				}
			}
			EventKind::Modify(_) => {
				for p in event.paths {
					if p == self.config.path {
						if let Ok(new_config) = Config::parse(&self.config.path) {
							if new_config != self.config {
								self.config = new_config;
								std::mem::drop(watcher);
								watcher = self.setup(tx);
								log::info!("Reloaded config");
							}
						}
					}
				}
			}
			_ => {}
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
