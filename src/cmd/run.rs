use crate::{Cmd, DEFAULT_CONFIG_STR};
use anyhow::Result;
use clap::Clap;
use lib::{config::UserConfig, file::File, utils::UnwrapRef};
use rayon::prelude::*;
use std::{fs, path::PathBuf};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, default_value = &DEFAULT_CONFIG_STR)]
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
		config
			.as_ref()
			.rules
			.path_to_rules
			.unwrap_ref()
			.par_iter()
			.map(|(path, _)| fs::read_dir(path).unwrap())
			.into_par_iter()
			.for_each(|dir| {
				dir.collect::<Vec<_>>().into_par_iter().for_each(|file| {
					let path = file.unwrap().path();
					if path.is_file() {
						let file = File::new(path);
						file.process(&config);
					}
				});
			});
		Ok(())
	}
}
