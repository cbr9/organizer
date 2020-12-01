use crate::{Cmd, CONFIG_PATH_STR};
use anyhow::Result;
use clap::Clap;
use notify::RecursiveMode;
use organize_core::{
	config::{AsMap, UserConfig},
	file::File,
	utils::UnwrapRef,
};
use rayon::prelude::*;
use std::{fs, path::PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, default_value = &CONFIG_PATH_STR)]
	pub(crate) config: PathBuf,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		match UserConfig::new(&self.config) {
			Ok(config) => self.start(config),
			Err(_) => std::process::exit(0),
		}
	}
}

impl<'a> Run {
	pub(crate) fn start(self, config: UserConfig) -> Result<()> {
		let paths = config
			.as_ref()
			.rules
			.path_to_rules
			.unwrap_ref()
			.iter()
			.map(|(path, _)| path)
			.collect::<Vec<_>>();

		let process = |entry: DirEntry| {
			if entry.path().is_file() {
				let file = File::new(entry.path());
				file.process(&config)
			}
		};

		paths.par_iter().for_each(|path| {
			let recursive: &RecursiveMode = config.rules.get(path);
			if recursive == &RecursiveMode::Recursive {
				WalkDir::new(path).follow_links(true).into_iter().filter_map(|e| e.ok()).for_each(process);
			} else {
				WalkDir::new(path)
					.max_depth(1) // only direct descendants
					.follow_links(true)
					.into_iter()
					.filter_map(|e| e.ok())
					.for_each(process);
			};
		});
		Ok(())
	}
}
