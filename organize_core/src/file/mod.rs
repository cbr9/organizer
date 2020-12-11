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
			match rule.actions.run(&path, data.get_apply_actions(*i, *j), simulate) {
				Ok(new_path) => {
					path = new_path;
					Ok(())
				}
				Err(e) => Err(e),
			}
		};
		match data.get_match() {
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
		map.get(&self.path)
			.iter()
			.filter(|(i, j)| {
				!data.should_ignore(&self.path, *i, *j) && data.config.rules[*i].filters.r#match(&self.path, data.get_apply_filters(*i, *j))
			})
			.collect::<Vec<_>>()
	}
}

