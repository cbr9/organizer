use crate::{
	data::{options::r#match::Match, path_to_rules::PathToRules, Data},
};
use std::path::PathBuf;
use crate::data::path_to_recursive::PathToRecursive;
use notify::RecursiveMode;

pub struct File {
	pub path: PathBuf,
}

impl File {
	pub fn new<T: Into<PathBuf>>(path: T) -> Self {
		Self { path: path.into() }
	}

	pub fn process<'a>(self, data: &'a Data, path_to_rules: &'a PathToRules, path_to_recursive: &'a PathToRecursive, simulate: bool) {
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
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				rules.into_iter().try_for_each(|(i, j)| process_rule(i, j)).ok();
			}
			Match::First => {
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				if let Some((i, j)) = rules.first() {
					process_rule(i, j).ok();
				}
			}
		}
	}

	pub fn get_matching_rules<'a>(&self, data: &'a Data, path_to_rules: &'a PathToRules, path_to_recursive: &'a PathToRecursive) -> Vec<&'a (usize, usize)> {
		let (key, value) = path_to_rules.get_key_value(&self.path);
		let (recursive, depth) = path_to_recursive.get(&key).unwrap();
		if recursive == &RecursiveMode::Recursive {
            let depth = depth.expect("folder is recursive but depth is not defined") as usize;
			if self.path.components().count() - key.components().count() > depth && depth != 0 {
				return Vec::with_capacity(0)
			}
		}
		value
			.iter()
			.filter(|(i, j)| {
				!data.should_ignore(&self.path, *i, *j) && data.config.rules[*i].filters.r#match(&self.path, data.get_apply_filters(*i, *j))
			})
			.collect::<Vec<_>>()
	}
}

