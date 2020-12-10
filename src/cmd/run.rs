use crate::{Cmd, CONFIG_PATH_STR};
use anyhow::Result;
use clap::Clap;
use notify::RecursiveMode;
use organize_core::{
	data::{path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
	file::File,
};
use rayon::prelude::*;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, default_value = &CONFIG_PATH_STR)]
	pub(crate) config: PathBuf,
	#[clap(long, short = 's', about = "Do not change any files, but get output on the hypothetical changes")]
	simulate: bool,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let data = Data::new()?;
		self.start(data)
	}
}

impl<'a> Run {
	pub(crate) fn start(self, data: Data) -> Result<()> {
		let path_to_recursive = PathToRecursive::new(&data);
		let path_to_rules = PathToRules::new(&data.config);

		let process = |entry: DirEntry| {
			if entry.path().is_file() {
				let file = File::new(entry.path());
				file.process(&data, &path_to_rules, self.simulate)
			}
		};

		path_to_rules.keys().collect::<Vec<_>>().par_iter().for_each(|path| {
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
