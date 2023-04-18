use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use organize_core::{
	data::{config::Config, path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
	file::File,
	logger::Logger,
};
use rayon::prelude::*;

use crate::Cmd;

#[derive(Parser, Debug)]
pub struct Run {
	#[arg(long, short = 'c')]
	pub(crate) config: Option<PathBuf>,
	#[arg(long, default_value_t = false)]
	pub(crate) no_color: bool,
}

impl Cmd for Run {
	fn run(mut self) -> Result<()> {
		Logger::setup(self.no_color)?;
		self.config = Some(self.config.unwrap_or_else(|| Config::path().unwrap()).canonicalize()?);
		let data = Data::new(self.config.clone().unwrap())?;
		self.start(data)
	}
}

impl<'a> Run {
	pub(crate) fn start(self, data: Data) -> Result<()> {
		let path_to_recursive = PathToRecursive::new(&data);
		let path_to_rules = PathToRules::new(&data.config);

		path_to_rules.par_iter().for_each(|(path, _)| {
			let recursive = path_to_recursive.get(path).unwrap();
			let walker = recursive.to_walker(path);
			walker.into_iter().filter_map(|e| e.ok()).for_each(|entry| {
				if entry.path().is_file() {
					let file = File::new(entry.path(), &data, false);
					file.act(&path_to_rules);
				}
			});
		});
		Ok(())
	}
}
