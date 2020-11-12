use crate::{Cmd, DEFAULT_CONFIG_STR};
use anyhow::Result;
use clap::Clap;
use lib::{
	config::{AsMap, Match, UserConfig},
	file::File,
	utils::UnwrapRef,
};
use rayon::prelude::*;
use std::{collections::HashMap, fs, path::PathBuf};

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
						let mut file = File::new(path);
						match config.defaults.unwrap_ref().r#match.unwrap_ref() {
							Match::All => file.get_matching_rules(config.as_ref()).into_iter().for_each(|(i, j)| {
								let rule = &config.rules[*i];
								rule.actions
									.run(&file.path, rule.folders[*j].options.unwrap_ref().apply.unwrap_ref().actions.unwrap_ref())
									.and_then(|f| {
										file.path = f;
										Ok(())
									});
							}),
							Match::First => {
								let rules = file.get_matching_rules(config.as_ref());
								if !rules.is_empty() {
									let (i, j) = rules.first().unwrap();
									let rule = &config.rules[*i];
									rule.actions
										.run(&file.path, rule.folders[*j].options.unwrap_ref().apply.unwrap_ref().actions.unwrap_ref())
										.and_then(|f| {
											file.path = f;
											Ok(())
										});
								}
							}
						}
					}
				});
			});
		Ok(())
	}
}
