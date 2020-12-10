use crate::{
	data::{options::r#match::Match, path_to_rules::PathToRules, Data},
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

	pub fn process<'a>(self, data: &'a Data, map: &'a PathToRules, simulate: bool) {
		let mut path = self.path.clone();
		let mut process_rule = |i: &usize, j: &usize| {
			let rule = &data.config.rules[*i];
			let folder = &rule.folders[*j];
			let actions = folder.options.apply.actions.as_ref().unwrap_or_else(|| {
				rule.options.apply.actions.as_ref().unwrap_or_else(|| {
					data.config.defaults.apply.actions.as_ref().unwrap_or_else(|| {
						data.settings
							.defaults
							.apply
							.actions
							.as_ref()
							.unwrap_or_else(|| data.defaults.apply.actions.unwrap_ref())
					})
				})
			});
			match rule.actions.run(&path, actions, simulate) {
				Ok(new_path) => {
					path = new_path;
					Ok(())
				}
				Err(e) => Err(e),
			}
		};
		let r#match = data.config.defaults.r#match.as_ref().unwrap_or_else(|| {
			data.settings
				.defaults
				.r#match
				.as_ref()
				.unwrap_or_else(|| data.defaults.r#match.unwrap_ref())
		});
		match r#match {
			Match::All => {
				let rules = self.get_matching_rules(data, map);
				rules.into_iter().try_for_each(|(i, j)| process_rule(i, j)).ok();
			}
			Match::First => {
				let rules = self.get_matching_rules(data, map);
				if let Some((i, j)) = rules.first() {
					process_rule(i, j).ok();
				}
			}
		}
	}

	pub fn get_matching_rules<'a>(&self, data: &'a Data, map: &'a PathToRules) -> Vec<&'a (usize, usize)> {
		let parent = self.path.parent().unwrap();
		let mut top_ignored: Vec<&PathBuf> = Vec::new(); // the default is an empty Vec
		if let Some(vec) = &data.settings.defaults.ignore {
			top_ignored.extend(vec);
		}
		if let Some(vec) = &data.config.defaults.ignore {
			top_ignored.extend(vec);
		}
		let possible_rules = map.get(&self.path);
		possible_rules
			.iter()
			.filter(|(i, j)| {
				let rule = &data.config.rules[*i];
				let folder = &rule.folders[*j];
				let hidden_files = folder.options.hidden_files.as_ref().unwrap_or_else(|| {
					rule.options.hidden_files.as_ref().unwrap_or_else(|| {
						data.config.defaults.hidden_files.as_ref().unwrap_or_else(|| {
							data.settings
								.defaults
								.hidden_files
								.as_ref()
								.unwrap_or_else(|| data.defaults.hidden_files.unwrap_ref())
						})
					})
				});
				let filters = folder.options.apply.filters.as_ref().unwrap_or_else(|| {
					rule.options.apply.filters.as_ref().unwrap_or_else(|| {
						data.config.defaults.apply.filters.as_ref().unwrap_or_else(|| {
							data.settings
								.defaults
								.apply
								.filters
								.as_ref()
								.unwrap_or_else(|| data.defaults.apply.filters.unwrap_ref())
						})
					})
				});
				let mut lower_ignored = Vec::new();
				if let Some(ignore) = &folder.options.ignore {
					lower_ignored.extend(ignore);
				}
				if let Some(ignore) = &rule.options.ignore {
					lower_ignored.extend(ignore);
				}

				if top_ignored.iter().any(|path| path == &parent) || lower_ignored.iter().any(|path| path == &parent) {
					return false;
				}
				if self.path.is_hidden() && !*hidden_files {
					return false;
				}
				rule.filters.r#match(&self.path, filters)
			})
			.collect::<Vec<_>>()
	}
}
