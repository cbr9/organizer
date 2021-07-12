use crate::{
	data::{options::r#match::Match, path_to_rules::PathToRules, Data},
	path::IsHidden,
	simulation::Simulation,
};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

pub struct File<'a> {
	pub path: PathBuf,
	data: &'a Data,
	is_watching: bool,
}

impl<'a> File<'a> {
	pub fn new<T: Into<PathBuf>>(path: T, data: &'a Data, is_watching: bool) -> Self {
		Self {
			path: path.into(),
			data,
			is_watching,
		}
	}

	pub fn simulate(mut self, path_to_rules: &'a PathToRules, simulation: &Arc<Mutex<Simulation>>) {
		let rules = self.get_matching_rules(path_to_rules);
		for (i, j) in rules {
			let rule = &self.data.config.rules[*i];
			match rule.actions.simulate(self.path, self.data.get_apply_actions(*i, *j), simulation) {
				None => break,
				Some(new_path) => {
					self.path = new_path;
				}
			}
		}
	}

	pub fn act(mut self, path_to_rules: &'a PathToRules) {
		let rules = self.get_matching_rules(path_to_rules);
		for (i, j) in rules {
			let rule = &self.data.config.rules[*i];
			match rule.actions.act(self.path, self.data.get_apply_actions(*i, *j)) {
				None => break,
				Some(new_path) => {
					self.path = new_path;
				}
			}
		}
	}

	fn filter_by_recursive<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		let depth = *self.data.get_recursive_depth(rule, folder) as usize;
		if depth == 0 {
			return true;
		}
		return self.path.components().count() - ancestor.as_ref().components().count() <= depth;
	}

	fn filter_by_partial_files(&self, rule: usize, folder: usize) -> bool {
		if !*self.data.allows_partial_files(rule, folder) {
			// if partial files are allowed
			if let Some(extension) = self.path.extension() {
				let partial_extensions = &["crdownload", "part"];
				let extension = extension.to_string_lossy();
				return !partial_extensions.contains(&&*extension);
			}
		}
		true
	}

	fn filter_by_hidden_files(&self, rule: usize, folder: usize) -> bool {
		(self.path.is_hidden() && *self.data.allows_hidden_files(rule, folder)) || !self.path.is_hidden()
	}

	fn filter_by_ignored_dirs(&self, rule: usize, folder: usize) -> bool {
		let check_ignored = |dir: &PathBuf| -> bool { self.path.parent().map(|parent| dir == parent).unwrap_or_default() };
		if let Some(ignored_dirs) = &self.data.settings.defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.rules[rule].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.rules[rule].folders[folder].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		true
	}

	fn filter_by_watch(&self, rule: usize, folder: usize) -> bool {
		!self.is_watching || *self.data.allows_watching(rule, folder)
	}

	fn filter_by_options<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		self.filter_by_recursive(ancestor, rule, folder)
			&& self.filter_by_hidden_files(rule, folder)
			&& self.filter_by_ignored_dirs(rule, folder)
			&& self.filter_by_partial_files(rule, folder)
			&& self.filter_by_watch(rule, folder)
	}

	fn filter_by_filters(&self, rule: usize, folder: usize) -> bool {
		let apply = self.data.get_apply_filters(rule, folder);
		let rule = &self.data.config.rules[rule];
		rule.filters.r#match(&self.path, apply)
	}

	fn filter<T: AsRef<Path>>(&self, ancestor: T, rule: &usize, folder: &usize) -> bool {
		let (rule, folder) = (*rule, *folder);
		self.filter_by_options(ancestor, rule, folder) && self.filter_by_filters(rule, folder)
	}

	pub fn get_matching_rules(&mut self, path_to_rules: &'a PathToRules) -> Vec<&'a (usize, usize)> {
		let (ancestor, rules) = path_to_rules.get_key_value(&self.path).unwrap();
		match self.data.match_rules() {
			Match::First => rules
				.iter()
				.find(|(rule, folder)| self.filter(ancestor, rule, folder))
				.map_or_else(Vec::new, |rule| vec![rule]),
			Match::All => rules.iter().filter(|(rule, folder)| self.filter(ancestor, rule, folder)).collect(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::file::File;
	use crate::{
		data::{
			folders::Folder, config::Config, config::Rule,
			options::{recursive::Recursive, Options},
			settings::Settings,
			Data,
		},
		utils::DefaultOpt,
	};
	use lazy_static::lazy_static;
	use std::ops::Deref;
	use std::path::PathBuf;

	lazy_static! {
		static ref DOWNLOADS: PathBuf = PathBuf::from("/Downloads");
		static ref DOCUMENTS: PathBuf = PathBuf::from("/Documents");
		static ref IGNORED_DIR_IN_RULE_1: &'static str = "ignored_dir_in_rule_1";
		static ref DATA: Data = {
			Data {
				defaults: Options::default_some(),
				settings: Settings {
					defaults: Options::default_none(),
				},
				config: Config {
					defaults: Options::default_none(),
					rules: vec![
						Rule {
							folders: vec![
								Folder {
									path: DOWNLOADS.clone(),
									options: Options {
										recursive: Recursive { depth: Some(2) },
										watch: Some(false),
										hidden_files: Some(true),
										partial_files: Some(false),
										ignored_dirs: Some(vec![DOWNLOADS.join(IGNORED_DIR_IN_RULE_1.deref())]),
										..Options::default_none()
									},
								},
								Folder {
									path: DOCUMENTS.clone(),
									options: Options {
										recursive: Recursive { depth: Some(5) },
										watch: Some(true),
										hidden_files: Some(false),
										partial_files: Some(true),
										ignored_dirs: Some(vec![DOCUMENTS.join(IGNORED_DIR_IN_RULE_1.deref())]),
										..Options::default_none()
									},
								},
							],
							..Rule::default()
						},
						Rule {
							folders: vec![
								Folder {
									path: DOWNLOADS.clone(),
									options: Options {
										recursive: Recursive { depth: Some(1) },
										watch: Some(true),
										hidden_files: Some(false),
										partial_files: Some(true),
										..Options::default_none()
									},
								},
								Folder {
									path: DOCUMENTS.clone(),
									options: Options {
										watch: Some(false),
										recursive: Recursive { depth: Some(0) },
										hidden_files: Some(true),
										partial_files: Some(false),
										..Options::default_none()
									},
								},
							],
							..Rule::default()
						},
					],
				},
			}
		};
	}

	#[test]
	fn filter_by_watch() {
		let mut file = File::new(DOWNLOADS.join("test.pdf"), DATA.deref(), true);
		assert!(!file.filter_by_watch(0, 0)); // at (0, 0), watch is Some(false)
		assert!(file.filter_by_watch(1, 0)); // at (1, 0), watch is Some(true)
		file.is_watching = false;
		assert!(file.filter_by_watch(0, 0)); // at (0, 0), watch is Some(false)
		assert!(file.filter_by_watch(1, 0)); // at (1, 0), watch is Some(true)
		file.path = DOCUMENTS.join("test.pdf");
		file.is_watching = true;
		assert!(file.filter_by_watch(0, 1)); // at (0, 1), watch is Some(true)
		assert!(!file.filter_by_watch(1, 1)); // at (1, 1), watch is Some(false)
		file.is_watching = false;
		assert!(file.filter_by_watch(0, 1)); // at (0, 1), watch is Some(true)
		assert!(file.filter_by_watch(1, 1)); // at (1, 1), watch is Some(false)
	}
	#[test]
	fn filter_by_recursive() {
		let dir = DOWNLOADS.join("depth1");
		let file = File::new(dir.join("depth2.pdf"), DATA.deref(), true);
		assert!(file.filter_by_recursive(DOWNLOADS.deref(), 0, 0)); // at (0, 0), recursive.depth is Some(2)
		assert!(!file.filter_by_recursive(DOWNLOADS.deref(), 1, 0)); // at (0, 0), recursive.depth is Some(1)
		let dir = DOCUMENTS.join("depth1").join("depth2").join("depth3").join("depth4").join("depth5");
		let file = File::new(dir.join("depth6.pdf"), DATA.deref(), true);
		assert!(!file.filter_by_recursive(DOWNLOADS.deref(), 0, 1)); // at (0, 1), recursive.depth is Some(5)
		assert!(file.filter_by_recursive(DOWNLOADS.deref(), 1, 1)); // at (1, 1), recursive.depth is Some(0)
	}
	#[test]
	fn filter_by_hidden_files() {
		let file = File::new(DOWNLOADS.join(".test.pdf"), DATA.deref(), true);
		assert!(file.filter_by_hidden_files(0, 0)); // at (0, 0), hidden_files is Some(true)
		assert!(!file.filter_by_hidden_files(1, 0)); // at (1, 0), hidden_files is Some(false)
		let file = File::new(DOCUMENTS.join("test.pdf"), DATA.deref(), true);
		assert!(file.filter_by_hidden_files(0, 1)); // at (0, 1), hidden_files is Some(false)
		assert!(file.filter_by_hidden_files(1, 1)); // at (1, 1), hidden_files is Some(true)
	}
	#[test]
	fn filter_by_partial_files() {
		let file = File::new(DOWNLOADS.join("test.part"), DATA.deref(), true);
		assert!(!file.filter_by_partial_files(0, 0)); // at (0, 0), partial_files is Some(false)
		assert!(file.filter_by_partial_files(1, 0)); // at (1, 0), partial_files is Some(true)
		let file = File::new(DOCUMENTS.join("test.pdf"), DATA.deref(), true);
		assert!(file.filter_by_partial_files(0, 1)); // at (0, 1), partial_files is Some(true)
		assert!(file.filter_by_partial_files(1, 1)); // at (1, 1), partial_files is Some(false)
	}
	#[test]
	fn filter_by_ignored_dirs() {
		let file = File::new(DOWNLOADS.join(IGNORED_DIR_IN_RULE_1.deref()).join("ignored.pdf"), DATA.deref(), true);
		assert!(!file.filter_by_ignored_dirs(0, 0)); // at (0, 0), ignored_dirs contains DOWNLOADS/IGNORED_DIR_IN_RULE_1
		assert!(file.filter_by_ignored_dirs(1, 0)); // at (1, 0), ignored_dirs does not contain DOWNLOADS/IGNORED_DIR_IN_RULE_1
		let file = File::new(DOWNLOADS.join("not_ignored.pdf"), DATA.deref(), true);
		assert!(file.filter_by_ignored_dirs(0, 0)); // at (0, 0), ignored_dirs contains DOWNLOADS/IGNORED_DIR_IN_RULE_1
		assert!(file.filter_by_ignored_dirs(1, 0)); // at (1, 0), ignored_dirs does not contain DOWNLOADS/IGNORED_DIR_IN_RULE_1
		let file = File::new(DOCUMENTS.join(IGNORED_DIR_IN_RULE_1.deref()).join("ignored.pdf"), DATA.deref(), true);
		assert!(!file.filter_by_ignored_dirs(0, 1)); // at (0, 1), ignored_dirs contains DOWNLOADS/IGNORED_DIR_IN_RULE_1
		assert!(file.filter_by_ignored_dirs(1, 1)); // at (1, 1), ignored_dirs does not contain DOWNLOADS/IGNORED_DIR_IN_RULE_1
		let file = File::new(DOCUMENTS.join("not_ignored.pdf"), DATA.deref(), true);
		assert!(file.filter_by_ignored_dirs(0, 1)); // at (0, 1), ignored_dirs contains DOWNLOADS/IGNORED_DIR_IN_RULE_1
		assert!(file.filter_by_ignored_dirs(1, 1)); // at (1, 1), ignored_dirs does not contain DOWNLOADS/IGNORED_DIR_IN_RULE_1
	}
}
