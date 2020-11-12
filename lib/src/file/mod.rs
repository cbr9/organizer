use crate::{
	config::{ApplyWrapper, AsMap, Options, Rule, UserConfig},
	path::{GetRules, IsHidden},
	utils::UnwrapRef,
};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

pub struct File {
	pub path: PathBuf,
}

impl File {
	pub fn new(path: PathBuf) -> Self {
		Self { path }
	}

	pub fn get_matching_rules<'a>(&self, config: &'a UserConfig) -> Vec<&'a (usize, usize)> {
		// let path2rules = config.as_ref().rules.map();
		let parent = self.path.parent().unwrap();
		let possible_rules: &Vec<(usize, usize)> = &config.rules.get(&self.path);
		possible_rules
			.iter()
			.filter(|(i, j)| {
				let rule = &config.rules[*i];
				let folder = &rule.folders[*j];
				match folder.options.unwrap_ref() {
					Options {
						ignore: Some(ignore),
						hidden_files: Some(hidden_files),
						apply: Some(ApplyWrapper { filters: Some(filters), .. }),
						..
					} => {
						if ignore.contains(&parent.to_path_buf()) {
							return false;
						}
						if self.path.is_hidden() && !*hidden_files {
							return false;
						}
						rule.filters.r#match(&self.path, filters)
					}
					_ => unreachable!(),
				}
			})
			.collect::<Vec<_>>()
	}
}
