use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
};

use actions::Action;
use anyhow::{Context, Result};
use options::apply::Apply;
use serde::Deserialize;

use crate::{
	utils::{DefaultOpt, UnwrapRef},
	PROJECT_NAME,
};

use self::{
	filters::Filters,
	folders::Folders,
	options::{r#match::Match, recursive::Recursive, Options},
};

pub mod actions;
pub mod filters;
pub mod folders;
pub mod options;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConfigBuilder {
	pub rules: Vec<Rule>,
	#[serde(rename = "defaults", default = "Options::default_some")]
	pub local_defaults: Options,
	#[serde(skip)]
	pub global_defaults: Options,
}

impl ConfigBuilder {
	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Self> {
		let path = path.as_ref();
		let s = fs::read_to_string(path)?;
		toml::from_str(&s).context("Could not deserialize config")
	}

	pub fn path_to_rules(&self) -> HashMap<PathBuf, Vec<(usize, usize)>> {
		let mut map = HashMap::with_capacity(self.rules.len()); // there will be at least one folder per rule
		self.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				map.entry(folder.path.to_path_buf()).or_insert_with(Vec::new).push((i, j));
			})
		});
		map.shrink_to_fit();
		map
	}

	pub fn path_to_recursive(&self) -> HashMap<PathBuf, Recursive> {
		let mut map = HashMap::with_capacity(self.rules.len());
		self.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let depth = *self.get_recursive_depth(i, j);
				map.entry(folder.path.to_path_buf())
					.and_modify(|entry: &mut Recursive| {
						if let Some(curr_depth) = entry.depth {
							if curr_depth != 0 && (depth == 0 || depth > curr_depth) {
								// take the greatest depth, except if it equals 0 or the current depth is already 0
								entry.depth = Some(depth);
							}
						}
					})
					.or_insert(Recursive { depth: Some(depth) });
			})
		});
		map.shrink_to_fit();
		map
	}
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
	pub rules: Vec<Rule>,
	pub path: PathBuf,
	pub local_defaults: Options,
	pub global_defaults: Options,
	pub path_to_rules: HashMap<PathBuf, Vec<(usize, usize)>>,
	pub path_to_recursive: HashMap<PathBuf, Recursive>,
}

macro_rules! getters {
	($($v:vis fn $name:ident(&self, rule: $rul:ty, folder: $fol:ty) -> $typ:ty {$field:tt})+) => {
		impl Config {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
				let rule = &self.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.as_ref().unwrap_or_else(|| {
					rule.options.$field.as_ref().unwrap_or_else(|| {
						self.local_defaults.$field.as_ref().unwrap_or_else(|| {
							self.global_defaults.$field.unwrap_ref()
						})
					})
				})
			})+
		}
		impl ConfigBuilder {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
				let rule = &self.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.as_ref().unwrap_or_else(|| {
					rule.options.$field.as_ref().unwrap_or_else(|| {
						self.local_defaults.$field.as_ref().unwrap_or_else(|| {
							self.global_defaults.$field.unwrap_ref()
						})
					})
				})
			})+
		}
	};
	($($v:vis fn $name:ident(&self) -> $typ:ty {$field:tt} )+) => {
		impl Config {
			$($v fn $name(&self) -> &$typ {
				self.local_defaults.$field.as_ref().unwrap_or_else(|| {
					self.global_defaults.$field.unwrap_ref()
				})
			})+
		}
		impl ConfigBuilder {
			$($v fn $name(&self) -> &$typ {
				self.local_defaults.$field.as_ref().unwrap_or_else(|| {
					self.global_defaults.$field.unwrap_ref()
				})
			})+
		}
	};
	($($v:vis fn $name:ident(&self, rule: $rul:ty, folder: $fol:ty) -> $typ:ty { $field:tt.$subfield:tt })+) => {
		impl Config {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
				let rule = &self.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.$subfield.as_ref().unwrap_or_else(|| {
					rule.options.$field.$subfield.as_ref().unwrap_or_else(|| {
						self.local_defaults.$field.$subfield.as_ref().unwrap_or_else(|| {
							self.global_defaults.$field.$subfield.unwrap_ref()
						})
					})
				})
			})+
		}
		impl ConfigBuilder {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
				let rule = &self.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.$subfield.as_ref().unwrap_or_else(|| {
					rule.options.$field.$subfield.as_ref().unwrap_or_else(|| {
						self.local_defaults.$field.$subfield.as_ref().unwrap_or_else(|| {
							self.global_defaults.$field.$subfield.unwrap_ref()
						})
					})
				})
			})+
		}
	};
}

getters! {
	pub fn match_rules(&self) -> Match {
		r#match
	}
}

getters! {
	pub fn allows_partial_files(&self, rule: usize, folder: usize) -> bool {
		partial_files
	}
	pub fn allows_hidden_files(&self, rule: usize, folder: usize) -> bool {
		hidden_files
	}
	pub fn get_apply(&self, rule: usize, folder: usize) -> Apply {
		apply
	}
}

getters! {
	pub fn get_recursive_depth(&self, rule: usize, folder: usize) -> u16 {
		recursive.depth
	}
}

impl Config {
	pub fn default_dir() -> PathBuf {
		let var = "ORGANIZE_CONFIG_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				dirs_next::config_dir()
					.unwrap_or_else(|| panic!("could not find config directory, please set {} manually", var))
					.join(PROJECT_NAME)
			},
			PathBuf::from,
		)
	}

	pub fn default_path() -> PathBuf {
		Self::default_dir().join("config.toml")
	}

	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Self> {
		let path = path.as_ref();
		let builder = ConfigBuilder::parse(path)?;
		Ok(Self {
			rules: builder.rules.clone(),
			local_defaults: builder.local_defaults.clone(),
			path: path.to_path_buf(),
			global_defaults: builder.global_defaults.clone(),
			path_to_rules: builder.path_to_rules(),
			path_to_recursive: builder.path_to_recursive(),
		})
	}

	pub fn path() -> Result<PathBuf> {
		std::env::current_dir()
			.context("Cannot determine current directory")?
			.read_dir()
			.context("Cannot determine directory content")?
			.find_map(|file| {
				let mut path = file.ok()?.path();
				if path.is_dir() && path.file_stem()?.to_string_lossy().ends_with(PROJECT_NAME) {
					return Some(path.join("config.toml"));
				} else if path.file_stem()?.to_string_lossy().ends_with(PROJECT_NAME) && path.extension()? == "toml" {
					return Some(path);
				}
				None
			})
			.map_or_else(
				|| Ok(Self::default_path()),
				|path| path.canonicalize().context("Couldn't find config file"),
			)
	}

	pub fn set_cwd<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
		let path = path.as_ref();
		if path == Self::default_path() {
			dirs_next::home_dir()
				.context("could not determine home directory")
				.and_then(|path| {
					std::env::set_current_dir(&path).context("Could not change into home directory")?;
					Ok(path)
				})
		} else {
			path.parent()
				.context("could not determine parent directory")
				.and_then(|path| {
					std::env::set_current_dir(path)?;
					Ok(path.to_path_buf())
				})
				.context("could not determine config directory")
		}
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Rule {
	pub name: Option<String>,
	#[serde(default)]
	pub tags: Vec<String>,
	pub actions: Vec<Action>,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "Options::default_none")]
	pub options: Options,
}

impl Default for Rule {
	fn default() -> Self {
		Self {
			name: None,
			tags: vec![],
			actions: vec![],
			filters: Filters(vec![]),
			folders: vec![],
			options: Options::default_none(),
		}
	}
}
