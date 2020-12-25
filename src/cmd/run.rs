use std::path::PathBuf;

use anyhow::Result;
use clap::Clap;
use notify::RecursiveMode;
use rayon::prelude::*;
use walkdir::{DirEntry, WalkDir};

use organize_core::{
	data::{Data, path_to_recursive::PathToRecursive, path_to_rules::PathToRules},
	file::File,
};
use organize_core::logger::Logger;

use crate::{Cmd, CONFIG_PATH_STR};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, short = 'c', default_value = & CONFIG_PATH_STR, about = "Config path")]
	pub(crate) config: PathBuf,
	#[clap(long, short = 's', about = "Do not change any files, but get output on the hypothetical changes")]
	pub(crate) simulate: bool,
	#[clap(long, about = "Do not print colored output")]
	pub(crate) no_color: bool,
}

impl Cmd for Run {
	fn run(mut self) -> Result<()> {
		Logger::setup(self.no_color)?;
		self.config = self.config.canonicalize()?;
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
				file.process(&data, &path_to_rules, &path_to_recursive, self.simulate)
			}
		};
		path_to_rules.keys().collect::<Vec<_>>().par_iter().for_each(|path| {
			let (recursive, depth) = path_to_recursive.get(path).unwrap();
			if recursive == &RecursiveMode::Recursive {
				let depth = depth.expect("depth is not defined but recursive is true");
				if depth == 0 {
					// no limit
					WalkDir::new(path).follow_links(true).into_iter().filter_map(|e| e.ok()).for_each(process);
				} else {
					WalkDir::new(path)
						.max_depth(depth as usize)
						.follow_links(true)
						.into_iter()
						.filter_map(|e| e.ok())
						.for_each(process);
				}
			} else {
				WalkDir::new(path)
					.max_depth(1) // only direct descendants, i.e. walk in a non recursive way
					.follow_links(true)
					.into_iter()
					.filter_map(|e| e.ok())
					.for_each(process);
			};
		});
		Ok(())
	}
}
