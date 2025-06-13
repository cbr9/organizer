use std::{
	collections::HashMap,
	fmt::Debug,
	fs::{File, OpenOptions},
	io::{BufWriter, Read, Seek, Write as Writer},
	ops::Deref,
	path::PathBuf,
	sync::{LazyLock, Mutex},
};

use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, Action};

static KNOWN_FILES: LazyLock<Mutex<HashMap<PathBuf, Mutex<File>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Default, Debug)]
#[serde(rename = "lowercase")]
pub enum WriteMode {
	#[default]
	Append,
	Prepend,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct Write {
	text: Template,
	outfile: Template,
	#[serde(default)]
	mode: WriteMode,
	#[serde(default)]
	clear_before_first_write: bool,
	#[serde(default = "r#true")]
	sort: bool,
	#[serde(default)]
	continue_with: ContinueWith,
}

#[derive(Deserialize, Serialize, Default, PartialEq, Eq, Clone, Debug)]
#[serde(rename = "kebab_case")]
pub enum ContinueWith {
	#[default]
	Original,
	WrittenFile,
}
fn r#true() -> bool {
	true
}

#[typetag::serde]
impl Action for Write {
	fn config(&self) -> ActionConfig {
		ActionConfig {
			requires_dest: true,
			parallelize: true,
		}
	}
	#[tracing::instrument(ret, err(Debug))]
	fn get_target_path(&self, res: &Resource) -> Result<Option<PathBuf>> {
		let path = prepare_target_path(&ConflictOption::Overwrite, res, &self.outfile, true)?;
		if let Some(path) = path.as_ref() {
			let mut lock = KNOWN_FILES.lock().unwrap();
			if !lock.contains_key(path) {
				let file = OpenOptions::new()
					.truncate(self.clear_before_first_write)
					.append(self.mode == WriteMode::Append)
					.create(true)
					.read(true)
					.open(path)?;

				lock.insert(path.clone(), Mutex::new(file));
			}
		}
		Ok(path)
	}

	// #[tracing::instrument(ret, err, skip(dest))]
	fn execute(&self, res: &Resource, dest: Option<PathBuf>, dry_run: bool) -> Result<Option<PathBuf>> {
		let path = dest.unwrap();

		if !dry_run {
			let mut text = self.text.render(&res.context)?;
			if self.mode == WriteMode::Prepend {
				let mut existing_content = std::fs::read_to_string(&path)?;
				if !existing_content.ends_with('\n') {
					existing_content += "\n";
				}
				text = existing_content + text.as_str();
			}

			{
				let lock = KNOWN_FILES.lock().unwrap();
				let file = lock.get(&path).expect("file should be there").lock().unwrap();
				let mut writer = BufWriter::new(file.deref());
				writeln!(writer, "{}", text)?;
				writer.flush()?;
			}
		}

		if self.continue_with == ContinueWith::WrittenFile {
			Ok(Some(path))
		} else {
			Ok(Some(res.path.clone()))
		}
	}

	fn on_finish(&self, _resources: &[Resource], dry_run: bool) -> Result<()> {
		let mut lock = KNOWN_FILES.lock().unwrap();
		if self.sort && !dry_run {
			for (_path, file) in lock.iter() {
				let mut file = file.lock().unwrap();
				file.seek(std::io::SeekFrom::Start(0))?;
				let mut buffer = String::new();
				file.read_to_string(&mut buffer)?;
				file.set_len(0)?;
				let mut writer = BufWriter::new(file.deref());
				write!(writer, "{}", buffer.lines().sorted_by_key(|s| s.to_lowercase()).join("\n"))?;
				writer.flush()?;
			}
		}

		lock.clear();
		Ok(())
	}
}
