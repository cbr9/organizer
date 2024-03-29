use std::path::PathBuf;

use anyhow::Result;
use clap::Clap;

use organize_core::{
	data::{path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
	file::File,
	logger::Logger,
	simulation::Simulation,
	utils::UnwrapRef,
};
use rayon::prelude::*;

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
		let data = Data::new(&self.config)?;
		self.start(data)
	}
}

impl<'a> Run {
	pub(crate) fn start(self, data: Data) -> Result<()> {
		let path_to_recursive = PathToRecursive::new(&data);
		let path_to_rules = PathToRules::new(&data.config);
		let simulation = if self.simulate { Some(Simulation::new()?) } else { None };

		path_to_rules.par_iter().for_each(|(path, _)| {
			let recursive = path_to_recursive.get(path).unwrap();
			let walker = recursive.to_walker(path);
			walker.into_iter().filter_map(|e| e.ok()).for_each(|entry| {
				if entry.path().is_file() {
					let file = File::new(entry.path(), &data, false);
					if self.simulate {
						file.simulate(&path_to_rules, simulation.unwrap_ref());
					} else {
						file.act(&path_to_rules);
					}
				}
			});
		});
		Ok(())
	}
}
