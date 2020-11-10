use crate::{cmd::watch::process_file, Cmd, DEFAULT_CONFIG_STR};
use anyhow::Result;
use clap::Clap;
use lib::config::{AsMap, UserConfig};
use rayon::prelude::*;
use std::{fs, path::PathBuf};

#[derive(Clap, Debug)]
pub struct Run {
	#[clap(long, default_value = &DEFAULT_CONFIG_STR)]
	pub(crate) config: PathBuf,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let config = UserConfig::new(&self.config);
		self.start(config)
	}
}

impl Run {
	pub(crate) fn start<T>(self, config: T) -> Result<()>
	where
		T: AsRef<UserConfig>,
	{
		let path2rules = config.as_ref().rules.map();
		path2rules
			.par_iter()
			.map(|(path, _)| fs::read_dir(path).unwrap())
			.into_par_iter()
			.for_each(|dir| {
				dir.collect::<Vec<_>>().into_par_iter().for_each(|file| {
					let path = file.unwrap().path();
					process_file(&path, &path2rules, false)
				});
			});
		Ok(())
	}
}
