use crate::{
	config::{
		options::{apply::wrapper::ApplyWrapper, r#match::Match, Options},
		AsMap,
		UserConfig,
	},
	path::IsHidden,
	utils::UnwrapRef,
};
use std::path::PathBuf;

pub struct File {
	pub path: PathBuf,
}

impl File {
	pub fn new<T: Into<PathBuf>>(path: T) -> Self {
		Self { path: path.into() }
	}

	pub fn process(self, config: &UserConfig) {
		let mut path = self.path.clone();
		let mut process_rule = |i: &usize, j: &usize| {
			let rule = &config.rules[*i];
			let apply = rule.folders[*j].options.unwrap_ref().apply.unwrap_ref();
			match rule.actions.run(&path, apply.actions.unwrap_ref()) {
				Ok(new_path) => {
					path = new_path;
					Ok(())
				}
				Err(e) => Err(e),
			}
		};
		match config.defaults.unwrap_ref().r#match.unwrap_ref() {
			Match::All => {
				self.get_matching_rules(config.as_ref())
					.into_iter()
					.try_for_each(|(i, j)| process_rule(i, j))
					.ok();
			}
			Match::First => {
				let rules = self.get_matching_rules(config.as_ref());
				if !rules.is_empty() {
					let (i, j) = rules.first().unwrap();
					process_rule(i, j).ok();
				}
			}
		}
	}

	pub fn get_matching_rules<'a>(&self, config: &'a UserConfig) -> Vec<&'a (usize, usize)> {
		let parent = self.path.parent().unwrap();
		let possible_rules: &Vec<(usize, usize)> = &config.rules.get(self.path.as_path());
		possible_rules
			.iter()
			.filter(|(i, j)| {
				let rule = &config.rules[*i];
				let folder = &rule.folders[*j];
				match folder.options.unwrap_ref() {
					Options {
						recursive: _,
						watch: _,
						ignore: Some(ignore),
						hidden_files: Some(hidden_files),
						r#match: _,
						apply: Some(ApplyWrapper { filters: Some(filters), .. }),
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
