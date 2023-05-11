use std::{
	path::{Path, PathBuf},
	sync::mpsc::Sender,
};

use anyhow::Result;
use clap::Parser;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use organize_core::{
	data::{config::Config, path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
	file::File,
};

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
			config: unsafe { self.config.unwrap_unchecked() },
			clean: unsafe { self.clean.unwrap_unchecked() },
			no_color: unsafe { self.no_color.unwrap_unchecked() },
		})
	}
}

#[derive(Debug)]
pub struct Watch {
	pub config: PathBuf,
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

impl<'a> Watch {
	fn cleanup(&self) -> Result<()> {
		let cmd = Run::builder()
			.config(Some(self.config.clone()))?
			.no_color(Some(self.no_color))
			.build()?;
		cmd.start()
	}

	fn on_create<T: AsRef<Path>>(&self, path: T, data: &Data, path_to_rules: &PathToRules) {
		let path = path.as_ref();
		let config_parent = self.config.parent().expect("Couldn't find config path");
		if let Some(parent) = path.parent() {
			if parent != config_parent && path.is_file() {
				let file = File::new(path, &data, true);
				file.act(&path_to_rules);
			}
		}
	}

	fn event_handler(
		&self,
		res: notify::Result<Event>,
		data: &mut Data,
		path_to_rules: &mut PathToRules,
		path_to_recursive: &mut PathToRecursive,
		mut watcher: RecommendedWatcher,
		tx: &Sender<notify::Result<Event>>,
	) -> RecommendedWatcher {
		let event = res.unwrap();
		match event.kind {
			notify::EventKind::Create(_) => {
				for p in event.paths {
					self.on_create::<PathBuf>(p, &data, &path_to_rules);
				}
			}
			EventKind::Modify(_) => {
				for p in event.paths {
					if p == self.config {
						if let Ok(new_config) = Config::parse(&self.config) {
							if new_config != data.config {
								data.config = new_config;
								*path_to_rules = PathToRules::new(data.config.clone());
								*path_to_recursive = PathToRecursive::new(data.clone());
								std::mem::drop(watcher);
								watcher = self.setup(&path_to_recursive, tx);
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

	fn setup(&self, path_to_recursive: &PathToRecursive, tx: &Sender<notify::Result<Event>>) -> RecommendedWatcher {
		let mut watcher = RecommendedWatcher::new(tx.clone(), notify::Config::default()).unwrap();

		for (folder, recursive) in path_to_recursive.iter() {
			watcher.watch(folder, recursive.type_()).unwrap();
		}

		if let Some(parent) = self.config.parent() {
			watcher.watch(parent, RecursiveMode::NonRecursive).unwrap();
		}
		watcher
	}

	fn start(self) -> Result<()> {
		let mut data = Data::new(self.config.clone()).unwrap();
		let mut path_to_rules = PathToRules::new(data.config.clone());
		let mut path_to_recursive = PathToRecursive::new(data.clone());
		let (tx, rx) = std::sync::mpsc::channel();
		let mut watcher = self.setup(&path_to_recursive, &tx);

		for res in &rx {
			watcher = self.event_handler(res, &mut data, &mut path_to_rules, &mut path_to_recursive, watcher, &tx);
		}

		Ok(())
	}
}
