use crate::{Cmd, CONFIG_PATH_STR};
use anyhow::Result;
use clap::Clap;
use notify::RecursiveMode;
use organize_core::{
	config::UserConfig,
	data::{Data, PathToRecursive, PathToRules},
	file::File,
};
use rayon::prelude::*;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, default_value = &CONFIG_PATH_STR)]
	pub(crate) config: PathBuf,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let data = Data::new();
		let path_to_rules = PathToRules::new(&data);
		let path_to_recursive = PathToRecursive::new(&data);
		self.start(data, path_to_rules, path_to_recursive)
	}
}

impl<'a> Run {
	pub(crate) fn start(self, data: Data, path_to_rules: PathToRules<'_>, path_to_recursive: PathToRecursive<'_>) -> Result<()> {
		let paths = path_to_rules.keys().collect::<Vec<_>>();

		let process = |entry: DirEntry| {
			if entry.path().is_file() {
				let file = File::new(entry.path());
				file.process(&data, &path_to_rules)
			}
		};

		paths.par_iter().for_each(|path| {
			let recursive = path_to_recursive.get(path).unwrap();
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
